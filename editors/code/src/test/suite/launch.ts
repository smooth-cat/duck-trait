import { runTests } from '@vscode/test-electron';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

async function main(): Promise<void> {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'duck-trait-ws-'));
  try {
    await runTests({
      // the locally installed VS Code — no download needed
      vscodeExecutablePath:
        '/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code',
      extensionDevelopmentPath: path.resolve(__dirname, '../..'),
      extensionTestsPath: path.resolve(__dirname, './index'),
      launchArgs: [
        '--disable-extensions',
        '--disable-workspace-trust',
        '--user-data-dir',
        path.join(workspace, 'profile'),
        workspace,
      ],
    });
  } catch (err) {
    console.error(err);
    process.exit(1);
  }
}

void main();
