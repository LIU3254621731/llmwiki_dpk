const { execSync } = require('child_process');
const fs = require('fs');

// Delete old ico
try { fs.unlinkSync('src-tauri/icons/icon.ico'); } catch(e) {}

// Try to use png-to-ico
try {
  const pngToIco = require('png-to-ico');
  if (typeof pngToIco === 'function') {
    pngToIco(['src-tauri/icons/Square310x310Logo.png'])
      .then(buf => {
        fs.writeFileSync('src-tauri/icons/icon.ico', buf);
        console.log('ICO created:', buf.length);
      })
      .catch(e => console.error(e));
  } else if (pngToIco.default) {
    pngToIco.default(['src-tauri/icons/Square310x310Logo.png'])
      .then(buf => {
        fs.writeFileSync('src-tauri/icons/icon.ico', buf);
        console.log('ICO created:', buf.length);
      })
      .catch(e => console.error(e));
  } else {
    console.log('png-to-ico API not available, trying CLI...');
    try {
      execSync('npx png-to-ico src-tauri/icons/Square310x310Logo.png src-tauri/icons/icon_new.ico', { stdio: 'pipe' });
      const buf = fs.readFileSync('src-tauri/icons/icon_new.ico');
      fs.writeFileSync('src-tauri/icons/icon.ico', buf);
      console.log('ICO created via CLI:', buf.length);
    } catch (e) {
      console.error('CLI failed:', e.message);
    }
  }
} catch (e) {
  console.error(e);
}
