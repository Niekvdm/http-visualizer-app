import sharp from 'sharp';
import pngToIco from 'png-to-ico';
import { writeFileSync, mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir = join(__dirname, 'icons');

// Ensure icons directory exists
if (!existsSync(iconsDir)) {
  mkdirSync(iconsDir, { recursive: true });
}

// Create a simple HTTP icon - blue gradient background with white "H" text concept
// Using a simple colored square as base
async function generateBaseIcon(size) {
  // Create a gradient-like icon with HTTP visualizer theme (blue/purple)
  const svg = `
    <svg width="${size}" height="${size}" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style="stop-color:#3B82F6;stop-opacity:1" />
          <stop offset="100%" style="stop-color:#8B5CF6;stop-opacity:1" />
        </linearGradient>
      </defs>
      <rect width="${size}" height="${size}" rx="${size * 0.15}" fill="url(#grad)"/>
      <text x="50%" y="55%" dominant-baseline="middle" text-anchor="middle"
            font-family="Arial, sans-serif" font-weight="bold" font-size="${size * 0.5}" fill="white">H</text>
    </svg>
  `;

  return sharp(Buffer.from(svg)).png().toBuffer();
}

async function main() {
  console.log('Generating icons...');

  // Generate PNG icons at various sizes
  const sizes = [32, 128, 256, 512, 1024];
  const pngPaths = {};

  for (const size of sizes) {
    const buffer = await generateBaseIcon(size);
    const filename = size === 256 ? '128x128@2x.png' :
                     size === 32 ? '32x32.png' :
                     size === 128 ? '128x128.png' :
                     size === 512 ? 'icon.png' :
                     `${size}x${size}.png`;
    const filepath = join(iconsDir, filename);
    writeFileSync(filepath, buffer);
    pngPaths[size] = filepath;
    console.log(`Created ${filename}`);
  }

  // Generate ICO file from multiple sizes
  try {
    const icoBuffer = await pngToIco([pngPaths[32], pngPaths[128], pngPaths[256]]);
    writeFileSync(join(iconsDir, 'icon.ico'), icoBuffer);
    console.log('Created icon.ico');
  } catch (err) {
    console.error('Failed to create ICO:', err.message);
    // Fallback: use single PNG for ICO
    const icoBuffer = await pngToIco([pngPaths[256]]);
    writeFileSync(join(iconsDir, 'icon.ico'), icoBuffer);
    console.log('Created icon.ico (fallback)');
  }

  // For macOS ICNS, we just create a placeholder note
  // ICNS requires special tooling, but Windows doesn't need it
  console.log('Note: icon.icns requires macOS tools to generate properly');

  // Create a placeholder .icns reference (empty file for now, Windows build won't need it)
  writeFileSync(join(iconsDir, 'icon.icns'), Buffer.alloc(0));

  console.log('Icon generation complete!');
}

main().catch(console.error);
