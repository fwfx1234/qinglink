import { createRequire } from 'node:module'
import { readFile } from 'node:fs/promises'
import { existsSync, statSync } from 'node:fs'
import { createHash } from 'node:crypto'
import path from 'node:path'

/**
 * 原位（in-process）构建所需的 Vue 单文件组件插件、样式/资源插件与裸模块解析器。
 *
 * 为什么不用 @vitejs/plugin-vue + vite build：Vite 在构建时会用 esbuild 派生子进程
 * 完成 TS / SFC 转换，而当前沙箱禁止 Node 派生带管道的子进程（spawn EPERM）。
 * 本插件只用 @vue/compiler-sfc 与 TypeScript 编译器 API，全部在当前进程内完成；
 * vite.config.ts 仍是开发与常规环境下的标准构建入口。
 */

const VUE_FILE = /\.vue$/
const TS_FILE = /\.m?ts$/
const CSS_FILE = /\.(css|less|scss|sass|styl)$/
const ASSET_FILE = /\.(png|jpe?g|gif|svg|webp|avif|ico|woff2?|ttf|otf|eot)$/

const MIME_TYPES = {
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.webp': 'image/webp',
  '.avif': 'image/avif',
  '.ico': 'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.otf': 'font/otf',
  '.eot': 'application/vnd.ms-fontobject',
}

/** 把无扩展名的别名路径补全为真实文件路径 */
function resolveWithExtension(target) {
  const candidates = [target, `${target}.ts`, `${target}.tsx`, `${target}.mts`, `${target}.js`, `${target}.vue`]
  for (const candidate of candidates) {
    if (existsSync(candidate) && statSync(candidate).isFile()) return candidate
  }
  for (const candidate of [`${target}/index.ts`, `${target}/index.js`]) {
    if (existsSync(candidate)) return candidate
  }
  return null
}

/** 收集 <script setup> 顶层绑定名，供模板编译器生成正确的渲染访问路径 */
function collectBindings(compiler, source) {
  const bindings = {}
  if (!source) return bindings
  const program = compiler.babelParse(source, {
    sourceType: 'module',
    plugins: ['typescript', 'jsx'],
  }).program

  const declare = (name, type) => {
    if (name) bindings[name] = type
  }

  /** 解构声明也要逐个收集：const { a, b } = x / const [c] = y */
  const declarePattern = (pattern, type) => {
    if (!pattern) return
    switch (pattern.type) {
      case 'Identifier':
        declare(pattern.name, type)
        return
      case 'ObjectPattern':
        for (const property of pattern.properties) {
          if (property.type === 'RestElement') declarePattern(property.argument, type)
          else declarePattern(property.value, type)
        }
        return
      case 'ArrayPattern':
        for (const element of pattern.elements) declarePattern(element, type)
        return
      case 'AssignmentPattern':
        declarePattern(pattern.left, type)
        return
      case 'RestElement':
        declarePattern(pattern.argument, type)
        return
      default:
        return
    }
  }

  const walk = (node) => {
    if (!node || typeof node.type !== 'string') return
    switch (node.type) {
      case 'ImportDeclaration':
        // 导入的绑定是常量，模板中不可被 v-model 赋值
        for (const specifier of node.specifiers) {
          declare(specifier.local?.name, 'setup-const')
        }
        return
      case 'VariableDeclaration':
        for (const declaration of node.declarations) {
          // setup-maybe-ref：可能是 ref，模板里允许 v-model 写入
          declarePattern(declaration.id, node.kind === 'let' ? 'setup-let' : 'setup-maybe-ref')
        }
        return
      case 'FunctionDeclaration':
      case 'ClassDeclaration':
        declare(node.id?.name, 'setup-const')
        return
      case 'ExportNamedDeclaration':
      case 'ExportDefaultDeclaration':
        walk(node.declaration)
        return
      default:
        return
    }
  }

  for (const node of program.body) walk(node)
  return bindings
}

/**
 * @param {{ root: string, styles: Map<string, string> }} context
 */
export function vuePlugin({ root, styles }) {
  const requireFromRoot = createRequire(path.join(root, 'package.json'))
  const compiler = requireFromRoot('@vue/compiler-sfc')
  const ts = requireFromRoot('typescript')

  let styleSeed = 0

  function transpile(code, id) {
    return ts.transpileModule(code, {
      fileName: id,
      compilerOptions: {
        target: ts.ScriptTarget.ES2020,
        module: ts.ModuleKind.ESNext,
        jsx: ts.JsxEmit.Preserve,
        isolatedModules: true,
        importsNotUsedAsValues: 'Remove',
        sourceMap: false,
      },
      reportDiagnostics: false,
    }).outputText
  }

  function compileVue(rawSource, id) {
    const { descriptor, errors } = compiler.parse(rawSource, { filename: id, sourceMap: false })
    if (errors.length > 0) {
      throw new Error(`[vue] ${id} 解析失败：${errors[0].message ?? String(errors[0])}`)
    }

    const scopeId = `data-v-${createHash('sha256').update(id).digest('hex').slice(0, 8)}`
    const hasScoped = descriptor.styles.some((style) => style.scoped)
    const isProd = process.env.NODE_ENV === 'production'

    const setupSource = descriptor.scriptSetup?.content ?? descriptor.script?.content ?? ''
    const bindings = collectBindings(compiler, setupSource)

    let scriptCode = ''
    if (descriptor.script || descriptor.scriptSetup) {
      scriptCode = compiler.compileScript(descriptor, {
        id,
        isProd,
        inlineTemplate: false,
        genDefaultAs: '__sfc__',
      }).content
    } else {
      // 纯模板组件：createApp/render 仍需一个组件对象承载
      scriptCode = 'const __sfc__ = {}'
    }

    let templateCode = ''
    if (descriptor.template) {
      const result = compiler.compileTemplate({
        // 注意：scopeId 通过 id 选项传入（compileTemplate 会自行补 data-v- 前缀）
        id: scopeId,
        filename: id,
        source: descriptor.template.content,
        scoped: hasScoped,
        slotted: descriptor.slotted,
        isProd,
        compilerOptions: {
          bindingMetadata: bindings,
        },
      })
      if (result.errors.length > 0) {
        throw new Error(`[vue] ${id} 模板编译失败：${result.errors[0].message ?? String(result.errors[0])}`)
      }
      // compileTemplate 产出的是 `export function render(...)`：转成具名函数后挂到组件上，
      // 等价于 @vitejs/plugin-vue 的 render 附加逻辑。
      templateCode = result.code.replace(/^export function render/m, 'function __sfc_render__')
      if (!templateCode.includes('function __sfc_render__')) {
        throw new Error(`[vue] ${id} 模板编译未产出 render 函数`)
      }
    }

    const styleImports = []
    for (const style of descriptor.styles) {
      const key = `${id}?vue-style=${styleSeed++}`
      const compiledStyle = compiler.compileStyle({
        id: scopeId,
        filename: id,
        source: style.content,
        scoped: style.scoped,
        isProd,
      })
      if (compiledStyle.errors.length > 0) {
        throw new Error(`[vue] ${id} 样式编译失败：${compiledStyle.errors[0].message ?? String(compiledStyle.errors[0])}`)
      }
      styles.set(key, compiledStyle.code)
      styleImports.push(`import ${JSON.stringify(key)}`)
    }

    const parts = [
      ...styleImports,
      scriptCode,
      templateCode,
      descriptor.template ? '__sfc__.render = __sfc_render__' : '',
      hasScoped ? `__sfc__.__scopeId = ${JSON.stringify(scopeId)}` : '',
      `__sfc__.__file = ${JSON.stringify(path.basename(id))}`,
      'export default __sfc__',
    ].filter(Boolean)

    return transpile(parts.join('\n'), id)
  }

  return {
    name: 'qinglink:vue-inprocess',

    resolveId(source, importer) {
      if (source.startsWith('\0')) return null
      if (source.startsWith('node:')) return { id: source, external: true }
      // tsconfig.json 的 paths 别名：@/* -> src/*（补全扩展名 / index 文件）
      if (source === '@' || source.startsWith('@/')) {
        const target = path.join(root, 'src', source.slice(2))
        const resolved = resolveWithExtension(target)
        if (resolved) return { id: resolved, external: false }
        return null
      }
      if (source.startsWith('.') || path.isAbsolute(source) || source.startsWith('/')) {
        // 相对 / 绝对路径：补全省略的扩展名（Rollup 默认只补 .js）
        if (path.extname(source) === '') {
          const base = source.startsWith('/')
            ? path.join(root, source.slice(1))
            : path.resolve(path.dirname(importer ?? path.join(root, 'src/main.ts')), source)
          const resolved = resolveWithExtension(base)
          if (resolved) return { id: resolved, external: false }
        }
        return null
      }
      // 裸模块交给 @rollup/plugin-node-resolve（需要 import/module 条件才能拿到 ESM 构建）
      return null
    },

    transform(code, id) {
      const [filePath, query] = id.split('?')
      if (query !== undefined && query.startsWith('vue-style')) return null
      if (VUE_FILE.test(filePath)) {
        return { code: compileVue(code, filePath), map: null }
      }
      if (TS_FILE.test(filePath) && !filePath.endsWith('.d.ts')) {
        return { code: transpile(code, filePath), map: null }
      }
      return null
    },
  }
}

/**
 * 样式与静态资源插件：CSS 统一汇总为 `dist/assets/styles.<hash>.css`，
 * 小于 4KB 的图片/字体内联为 data URL，其余作为独立 asset 输出。
 *
 * 样式文件名带内容哈希是必须的：Rust 端对 `/assets/**` 一律发
 * `Cache-Control: immutable`（一年），固定名 `styles.css` 会让重建后的样式
 * 永远拿不到浏览器。哈希名与 Vite 构建的行为一致。
 *
 * @param {{ styles: Map<string, string>, emitted?: { stylesheet?: string } }} context
 */
export function assetsPlugin({ styles, emitted = {} }) {
  const collected = []
  const binaryAssets = new Map()

  return {
    name: 'qinglink:assets',

    async load(id) {
      const filePath = id.split('?')[0]
      if (styles.has(id)) {
        collected.push({ file: filePath, source: styles.get(id) ?? '' })
        return { code: '', moduleSideEffects: true }
      }
      if (CSS_FILE.test(filePath)) {
        collected.push({ file: filePath, source: await readFile(filePath, 'utf8') })
        return { code: '', moduleSideEffects: true }
      }
      if (ASSET_FILE.test(filePath)) {
        const buffer = await readFile(filePath)
        const extension = path.extname(filePath).toLowerCase()
        if (buffer.byteLength <= 4096) {
          const mime = MIME_TYPES[extension] ?? 'application/octet-stream'
          return { code: `export default ${JSON.stringify(`data:${mime};base64,${buffer.toString('base64')}`)}` }
        }
        binaryAssets.set(filePath, {
          buffer,
          name: `assets/${path.basename(filePath, extension)}.[hash:12]${extension}`,
        })
        return null
      }
      return null
    },

    generateBundle() {
      if (collected.length > 0) {
        // 汇总为单一入口样式表，保证令牌与基础样式先于组件样式生效
        const css = collected.map((entry) => entry.source).join('\n')
        const hash = createHash('sha256').update(css).digest('hex').slice(0, 12)
        const fileName = `assets/styles.${hash}.css`
        this.emitFile({ type: 'asset', fileName, source: css })
        emitted.stylesheet = `/${fileName}`
      }
      for (const [, value] of binaryAssets) {
        this.emitFile({ type: 'asset', fileName: value.name, source: value.buffer })
      }
    },
  }
}
