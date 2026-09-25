const sharp = require('C:/Users/fwfx1234/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/sharp');
const fs = require('fs');
const path = require('path');

const DESIGN = path.resolve(__dirname);
const ROUNDED = path.join(DESIGN, 'icon_v4.svg');      // 圆角：Windows ico、预览
const BLEED = path.join(DESIGN, 'icon_v4_bleed.svg');  // 全出血：Android legacy、iOS

const APP = path.resolve(__dirname, '..', '..');
const ANDROID_RES = path.join(APP, 'android/app/src/main/res');
const IOS_DIR = path.join(APP, 'ios/Runner/Assets.xcassets/AppIcon.appiconset');
const WINDOWS_ICO = path.join(APP, 'windows/runner/resources/app_icon.ico');

function circleMask(size) {
  return Buffer.from(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}">` +
    `<circle cx="${size/2}" cy="${size/2}" r="${size/2}" fill="#000"/></svg>`
  );
}

async function render(src, size, out, { maskCircle = false, removeAlpha = false } = {}) {
  let img = sharp(src).resize(size, size, { fit: 'fill' });
  if (maskCircle) img = img.composite([{ input: circleMask(size), blend: 'dest-in' }]);
  if (removeAlpha) img = img.removeAlpha();
  img = img.png();
  await img.toFile(out);
  console.log('  wrote', path.relative(APP, out), `(${size}x${size})`);
}

async function writeIco(src, out) {
  const sizes = [256, 48, 32, 16];
  const images = await Promise.all(sizes.map((size) => sharp(src).resize(size, size, { fit: 'fill' }).png().toBuffer()));
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);
  const entries = Buffer.alloc(images.length * 16);
  let offset = header.length + entries.length;
  images.forEach((image, i) => {
    const size = sizes[i] === 256 ? 0 : sizes[i];
    const entry = entries.subarray(i * 16, i * 16 + 16);
    entry[0] = size;
    entry[1] = size;
    entry[2] = 0;
    entry[3] = 0;
    entry.writeUInt16LE(1, 4);
    entry.writeUInt16LE(32, 6);
    entry.writeUInt32LE(image.length, 8);
    entry.writeUInt32LE(offset, 12);
    offset += image.length;
  });
  await fs.promises.writeFile(out, Buffer.concat([header, entries, ...images]));
  console.log('  wrote', path.relative(APP, out), '(16/32/48/256)');
}

(async () => {
  const androidSizes = [['mdpi',48],['hdpi',72],['xhdpi',96],['xxhdpi',144],['xxxhdpi',192]];
  console.log('== Android legacy icons (full square) ==');
  for (const [dpi, size] of androidSizes) {
    const dir = path.join(ANDROID_RES, `mipmap-${dpi}`);
    await render(BLEED, size, path.join(dir, 'ic_launcher.png'), { removeAlpha: true });
    await render(BLEED, size, path.join(dir, 'ic_launcher_round.png'), { maskCircle: true });
  }

  const iosSizes = [
    ['Icon-App-20x20@1x.png',20],['Icon-App-20x20@2x.png',40],['Icon-App-20x20@3x.png',60],
    ['Icon-App-29x29@1x.png',29],['Icon-App-29x29@2x.png',58],['Icon-App-29x29@3x.png',87],
    ['Icon-App-40x40@1x.png',40],['Icon-App-40x40@2x.png',80],['Icon-App-40x40@3x.png',120],
    ['Icon-App-60x60@2x.png',120],['Icon-App-60x60@3x.png',180],
    ['Icon-App-76x76@1x.png',76],['Icon-App-76x76@2x.png',152],
    ['Icon-App-83.5x83.5@2x.png',167],
    ['Icon-App-1024x1024@1x.png',1024],
  ];
  console.log('== iOS app icons (full bleed) ==');
  for (const [file, size] of iosSizes) {
    await render(BLEED, size, path.join(IOS_DIR, file), { removeAlpha: true });
  }
  console.log('== Windows icon ==');
  await writeIco(ROUNDED, WINDOWS_ICO);
  await render(ROUNDED, 1024, path.join(DESIGN, 'icon_v4_preview.png'));
  console.log('done');
})().catch(e => { console.error(e); process.exit(1); });

