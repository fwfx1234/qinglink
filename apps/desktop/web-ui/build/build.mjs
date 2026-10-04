// 生产构建的无子进程回退实现。
//
// 标准路径是 `npm run build`（vite build）。本脚本是它的等价替代，供
// 无法派生 Node 子进程的环境使用——例如受限的执行沙箱会阻止 esbuild 启动
// 它的服务进程，而 Vite 的 TS/SFC 转换依赖 esbuild。
//
// 本脚本用 Rollup 的程序化 API + 自带的 in-process Vue 插件复刻生产构建：
// 解析 @/ 别名、TS 转译、SFC compileScript/compileTemplate/compileStyle（含
// scoped）、虚拟 CSS 模块汇总为单文件、打包期常量替换，最后用 terser 的
// JS API（纯 JS，同样不派生子进程）压缩。
//
// 两条路径产出形状相同的 dist/，因此后端的内嵌逻辑无需感知用的是哪一条。
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { readdir } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { rollup } from 'rollup'
import replace from '@rollup/plugin-replace'
import { nodeResolve } from '@rollup/plugin-node-resolve'
import { minify } from 'terser'
import { assetsPlugin, vuePlugin } from './vue-plugin.mjs'

const root = path.resolve(fileURLToPath(new URL('..', import.meta.url)))
const outDir = path.join(root, 'dist')

process.env.NODE_ENV = 'production'

async function main() {
  await rm(outDir, { recursive: true, force: true })
  await mkdir(path.join(outDir, 'assets'), { recursive: true })

  const styles = new Map()
  // assetsPlugin 写入实际产出的样式表 URL（带内容哈希），供下面生成 <link>。
  const emitted = {}

  const bundle = await rollup({
    input: path.join(root, 'src/main.ts'),
    plugins: [
      replace({
        preventAssignment: true,
        values: {
          __DEV__: 'false',
          __FEATURE_OPTIONS_API__: 'true',
          __FEATURE_PROD_DEVTOOLS__: 'false',
          __FEATURE_SUSPENSE__: 'true',
          __VUE_OPTIONS_API__: 'true',
          __VUE_PROD_DEVTOOLS__: 'false',
          __VUE_PROD_HYDRATION_MISMATCH_DETAILS__: 'false',
          'process.env.NODE_ENV': JSON.stringify('production'),
        },
      }),
      vuePlugin({ root, styles }),
      nodeResolve({
        browser: true,
        moduleDirectories: ['node_modules'],
        mainFields: ['module', 'browser', 'jsnext:main', 'main'],
        exportConditions: ['import', 'module', 'browser', 'default'],
        extensions: ['.mjs', '.js', '.json'],
        preferBuiltins: false,
      }),
      assetsPlugin({ styles, emitted }),
    ],
    onwarn(warning, warn) {
      if (warning.code === 'CIRCULAR_DEPENDENCY') return
      warn(warning)
    },
  })

  await bundle.write({
    dir: outDir,
    format: 'es',
    entryFileNames: 'assets/app.[hash:12].js',
    chunkFileNames: 'assets/[name].[hash:12].js',
    assetFileNames: 'assets/[name].[hash:12][extname]',
    sourcemap: false,
    // 输出插件：用 terser 的 JS API 压缩。刻意不用 @rollup/plugin-terser，
    // 它通过 worker 并行压缩，而本脚本的存在前提就是不派生子进程。
    plugins: [
      {
        name: 'in-process-terser',
        async renderChunk(code) {
          const result = await minify(code, {
            module: true,
            ecma: 2020,
            compress: { passes: 2, drop_debugger: true, pure_getters: true },
            // 保留 license 注释之外的一切都去掉；Vue 的 MIT 声明通过 banner 保留。
            format: { comments: false },
          })
          if (typeof result.code !== 'string') {
            throw new Error('terser 未返回压缩结果')
          }
          return { code: result.code, map: null }
        },
      },
    ],
  })
  await bundle.close()

  const files = await collectFiles(outDir)
  const entryFile = files.find((file) => /^assets[\\/]app\..+\.js$/.test(file))
  if (!entryFile) throw new Error('构建未产出入口 chunk')
  const entryBytes = (await readFile(path.join(outDir, entryFile))).length
  if (entryBytes > 400 * 1024) {
    throw new Error(`入口 chunk 压缩后仍有 ${entryBytes} 字节，压缩很可能没有生效`)
  }

  const entryUrl = `/${entryFile.split(path.sep).join('/')}`
  const template = await readFile(path.join(root, 'index.html'), 'utf8')
  const styleTag = emitted.stylesheet
    ? `    <link rel="stylesheet" href="${emitted.stylesheet}" />\n`
    : ''
  const scriptTag = `${styleTag}    <script type="module" src="${entryUrl}"></script>`

  // index.html 里用 <!--vite-entry--> 标记产物应插入的位置（Vite 会保留注释并忽略它），
  // 再移除模板中的源码入口 <script src="/src/main.ts">。
  const withEntry = template.includes('<!--vite-entry-->')
    ? template.replace('<!--vite-entry-->', scriptTag)
    : template.replace('</head>', `${scriptTag}\n  </head>`)
  const html = withEntry.replace(/\s*<script type="module" src="\/src\/main\.ts"><\/script>/, '')
  await writeFile(path.join(outDir, 'index.html'), html)

  console.log('[build] 入口：', entryUrl)
  console.log('[build] 样式：', emitted.stylesheet ?? '（无独立样式文件）')
  console.log('[build] 产物清单：')
  for (const file of files) {
    console.log('        -', file.split(path.sep).join('/'))
  }
}

async function collectFiles(directory, prefix = '') {
  const entries = await readdir(directory, { withFileTypes: true })
  const results = []
  for (const entry of entries) {
    const relative = prefix ? path.join(prefix, entry.name) : entry.name
    if (entry.isDirectory()) {
      results.push(...(await collectFiles(path.join(directory, entry.name), relative)))
    } else {
      results.push(relative)
    }
  }
  return results
}

main().catch((error) => {
  console.error('[build] 失败：', error)
  process.exitCode = 1
})
