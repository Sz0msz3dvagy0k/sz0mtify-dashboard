import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';

const cacheRoot = process.env.XDG_CACHE_HOME ?? join(homedir(), '.cache');
const pluginPath = join(cacheRoot, 'tauri', 'linuxdeploy-plugin-gtk.sh');
const pluginUrl = 'https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh';

if (!existsSync(pluginPath)) {
	console.log(`downloading linuxdeploy GTK plugin to ${pluginPath}`);
	const response = await fetch(pluginUrl);
	if (!response.ok) {
		throw new Error(`failed to download ${pluginUrl}: ${response.status} ${response.statusText}`);
	}
	mkdirSync(dirname(pluginPath), { recursive: true });
	writeFileSync(pluginPath, await response.text());
	chmodSync(pluginPath, 0o755);
}

let source = readFileSync(pluginPath, 'utf8');
let patched = source;

patched = patched.replace(
	'done < <(find "$directory" \\( -type l -o -type f \\) -name "$library" -print0)',
	'done < <(find "$directory" -path /usr/lib/vmware -prune -o \\( -type l -o -type f \\) -name "$library" -print0)'
);

patched = patched.replace(
	'find /usr/lib* -name libgiognutls.so -exec mkdir -p "$APPDIR"/"$(dirname \'{}\')" \\; -exec cp --parents \'{}\' "$APPDIR/" \\; || true',
	'find /usr/lib* -path /usr/lib/vmware -prune -o -name libgiognutls.so -exec mkdir -p "$APPDIR"/"$(dirname \'{}\')" \\; -exec cp --parents \'{}\' "$APPDIR/" \\; || true'
);

if (patched === source) {
	console.log('linuxdeploy GTK plugin already patched');
} else {
	writeFileSync(pluginPath, patched);
	console.log(`patched ${pluginPath}`);
}
