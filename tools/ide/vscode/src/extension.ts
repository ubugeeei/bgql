import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const config = vscode.workspace.getConfiguration('bgql');

  // Start language server if enabled
  if (config.get<boolean>('lsp.enabled', true)) {
    await startLanguageServer(context);
  }

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('bgql.restartServer', async () => {
      if (client) {
        await client.stop();
      }
      await startLanguageServer(context);
      vscode.window.showInformationMessage('Better GraphQL language server restarted');
    }),

    vscode.commands.registerCommand('bgql.formatDocument', async () => {
      const editor = vscode.window.activeTextEditor;
      if (editor && editor.document.languageId === 'bgql') {
        await vscode.commands.executeCommand('editor.action.formatDocument');
      }
    }),

    vscode.commands.registerCommand('bgql.generateTypes', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== 'bgql') {
        vscode.window.showWarningMessage('Please open a .bgql file first');
        return;
      }

      const options = await vscode.window.showQuickPick(
        [
          { label: 'TypeScript', description: 'Generate TypeScript types' },
          { label: 'Rust', description: 'Generate Rust types' },
          { label: 'Go', description: 'Generate Go types' },
        ],
        { placeHolder: 'Select target language' }
      );

      if (options) {
        const terminal = vscode.window.createTerminal('bgql codegen');
        const filePath = editor.document.uri.fsPath;
        terminal.sendText(`bgql codegen --lang ${options.label.toLowerCase()} ${filePath}`);
        terminal.show();
      }
    })
  );

  // Status bar item
  const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  statusBarItem.text = '$(symbol-misc) bgql';
  statusBarItem.tooltip = 'Better GraphQL';
  statusBarItem.command = 'bgql.restartServer';
  context.subscriptions.push(statusBarItem);

  // Show status bar for .bgql files
  vscode.window.onDidChangeActiveTextEditor((editor) => {
    if (editor && editor.document.languageId === 'bgql') {
      statusBarItem.show();
    } else {
      statusBarItem.hide();
    }
  });

  // Check initial editor
  if (vscode.window.activeTextEditor?.document.languageId === 'bgql') {
    statusBarItem.show();
  }
}

async function startLanguageServer(context: vscode.ExtensionContext): Promise<void> {
  const config = vscode.workspace.getConfiguration('bgql');
  const serverPath = config.get<string>('lsp.path', 'bgql');

  const serverOptions: ServerOptions = {
    run: {
      command: serverPath,
      args: ['lsp'],
    } as Executable,
    debug: {
      command: serverPath,
      args: ['lsp', '--debug'],
    } as Executable,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'bgql' }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.bgql'),
    },
    outputChannelName: 'Better GraphQL',
    initializationOptions: {
      tabSize: config.get<number>('format.tabSize', 2),
      validation: config.get<boolean>('validation.enabled', true),
    },
  };

  client = new LanguageClient(
    'bgql',
    'Better GraphQL Language Server',
    serverOptions,
    clientOptions
  );

  try {
    await client.start();
    context.subscriptions.push(client);
  } catch (error) {
    vscode.window.showWarningMessage(
      `Failed to start Better GraphQL language server. Make sure 'bgql' is installed and in your PATH.`
    );
  }
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
  }
}
