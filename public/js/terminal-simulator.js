/**
 * Tuxpedia Terminal Simulator
 * A lightweight, mock Linux terminal that runs entirely in the browser.
 */

// ===== Virtual Filesystem =====
const virtualFS = {
    '/': {
        type: 'dir',
        children: {
            'home': { type: 'dir', children: {
                'user': { type: 'dir', children: {
                    'Documents': { type: 'dir', children: {
                        'notes.txt': { type: 'file', content: 'This is a sample document.' }
                    } },
                    'Downloads': { type: 'dir', children: {} },
                    '.bashrc': { type: 'file', content: 'alias ll="ls -la"\nexport PS1="\\u@tuxpedia:\\w$ "' }
                } }
            } },
            'etc': { type: 'dir', children: {
                'os-release': { type: 'file', content: 'NAME="Tuxpedia Linux"\nVERSION="0.1.0"\nID=tuxpedia\nPRETTY_NAME="Tuxpedia Linux 0.1.0"' }
            } },
            'bin': { type: 'dir', children: {} },
            'usr': { type: 'dir', children: {} },
            'var': { type: 'dir', children: {} },
            'tmp': { type: 'dir', children: {} },
            'root': { type: 'dir', children: {} },
            'README.md': { type: 'file', content: '# Welcome to Tuxpedia\n\nThis is a simulated Linux environment.' },
            'hello.txt': { type: 'file', content: 'Hello from Tuxpedia!' },
        }
    }
};

// ===== Utility Functions =====
function resolvePath(path, cwd) {
    if (path === '' || path === '.') {
        return cwd;
    }
    if (path === '..') {
        const parts = cwd.split('/').filter(p => p !== '');
        parts.pop();
        return '/' + parts.join('/');
    }
    if (path.startsWith('/')) {
        // Absolute path
        return path;
    }
    // Relative path
    return cwd + '/' + path;
}

function getNode(path, fs) {
    if (path === '/') {
        return fs['/'];
    }
    const parts = path.split('/').filter(p => p !== '');
    let current = fs['/'];
    for (const part of parts) {
        if (!current.children || !current.children[part]) {
            return null;
        }
        current = current.children[part];
    }
    return current;
}

function getParentPath(path) {
    const parts = path.split('/').filter(p => p !== '');
    parts.pop();
    return '/' + parts.join('/');
}

// ===== Command Handlers =====
const commands = {
    ls: function(args, env) {
        const target = args[0] || '.';
        const resolved = resolvePath(target, env.cwd);
        const node = getNode(resolved, virtualFS);
        if (!node) {
            return `ls: cannot access '${target}': No such file or directory`;
        }
        if (node.type !== 'dir') {
            return target;
        }
        const children = Object.keys(node.children || {});
        if (children.length === 0) {
            return '';
        }
        return children.join('  ');
    },

    pwd: function(args, env) {
        return env.cwd;
    },

    cd: function(args, env) {
        const target = args[0] || '/home/user';
        const resolved = resolvePath(target, env.cwd);
        const node = getNode(resolved, virtualFS);
        if (!node || node.type !== 'dir') {
            return `cd: ${target}: No such file or directory`;
        }
        env.cwd = resolved;
        return '';
    },

    echo: function(args, env) {
        return args.join(' ');
    },

    cat: function(args, env) {
        if (args.length === 0) {
            return 'cat: missing file operand';
        }
        const target = args[0];
        const resolved = resolvePath(target, env.cwd);
        const node = getNode(resolved, virtualFS);
        if (!node) {
            return `cat: ${target}: No such file or directory`;
        }
        if (node.type !== 'file') {
            return `cat: ${target}: Is a directory`;
        }
        return node.content || '';
    },

    whoami: function(args, env) {
        return 'user';
    },

    uname: function(args, env) {
        return 'Linux tuxpedia 6.1.0 #1 SMP PREEMPT_DYNAMIC x86_64 GNU/Linux';
    },

    clear: function(args, env) {
        return 'CLEAR_SCREEN';
    },

    exit: function(args, env) {
        return 'EXIT';
    },

    help: function(args, env) {
        return `
Available commands:
  ls [path]       - List directory contents
  cd [path]       - Change directory
  pwd             - Print working directory
  echo [text]     - Display text
  cat [file]      - Display file contents
  whoami          - Print current user
  uname           - Print system information
  clear           - Clear the terminal
  exit            - Exit the terminal
  help            - Show this help
`.trim();
    }
};

// ===== Terminal Initialization =====
function initTerminal(containerId) {
    const container = document.getElementById(containerId);
    if (!container) {
        console.error('Terminal container not found.');
        return;
    }

    // Check if terminal already initialized
    if (container._terminal) {
        return;
    }

    const term = new Terminal({
        cursorBlink: true,
        theme: {
            background: '#1e1e1e',
            foreground: '#d4d4d4',
            cursor: '#ffffff',
            selection: '#264f78',
            black: '#000000',
            red: '#cd3131',
            green: '#0dbc79',
            yellow: '#e5e510',
            blue: '#2472c8',
            magenta: '#bc3fbc',
            cyan: '#11a8cd',
            white: '#e5e5e5',
            brightBlack: '#666666',
            brightRed: '#f14c4c',
            brightGreen: '#23d18b',
            brightYellow: '#f5f543',
            brightBlue: '#3b8eea',
            brightMagenta: '#d670d6',
            brightCyan: '#29b8db',
            brightWhite: '#e5e5e5'
        },
        fontFamily: 'monospace',
        fontSize: 14,
    });

    const fitAddon = new FitAddon.FitAddon();
    term.loadAddon(fitAddon);

    term.open(container);
    fitAddon.fit();

    // Save reference to prevent double init
    container._terminal = term;

    // ===== Terminal State =====
    const env = {
        cwd: '/home/user',
        prompt: 'user@tuxpedia:~$ '
    };

    let currentInput = '';
    let history = [];
    let historyIndex = -1;
    let isExited = false;

    // ===== Input Handling =====
    function writePrompt() {
        term.write('\r\n' + env.prompt);
    }

    function processCommand(input) {
        if (input.trim() === '') {
            return '';
        }

        const parts = input.trim().split(/\s+/);
        const cmd = parts[0];
        const args = parts.slice(1);

        if (cmd === 'exit') {
            isExited = true;
            term.write('\r\nGoodbye! 🐧');
            setTimeout(() => {
                term.dispose();
                container._terminal = null;
                container.innerHTML = '<p class="text-center text-gray-400 p-4">Terminal session ended. <button onclick="location.reload()" class="text-blue-500 hover:underline">Reload</button></p>';
            }, 1000);
            return '';
        }

        if (commands[cmd]) {
            const output = commands[cmd](args, env);
            if (output === 'CLEAR_SCREEN') {
                term.clear();
                return '';
            }
            return output;
        } else {
            return `bash: ${cmd}: command not found`;
        }
    }

    // ===== Key Events =====
    term.onKey((e) => {
        const key = e.key;
        const ev = e.domEvent;

        if (isExited) return;

        if (ev.key === 'Enter') {
            term.write('\r\n');
            const output = processCommand(currentInput);
            if (output !== '') {
                term.writeln(output);
            }
            if (!isExited) {
                writePrompt();
            }
            if (currentInput.trim() !== '' && currentInput.trim() !== 'exit') {
                history.push(currentInput);
            }
            historyIndex = history.length;
            currentInput = '';
            return;
        }

        if (ev.key === 'Backspace') {
            if (currentInput.length > 0) {
                currentInput = currentInput.slice(0, -1);
                term.write('\b \b');
            }
            return;
        }

        if (ev.key === 'ArrowUp') {
            if (history.length > 0) {
                if (historyIndex > 0) {
                    historyIndex--;
                }
                const prev = history[historyIndex];
                if (prev !== undefined) {
                    // Clear current input
                    term.write('\b \b'.repeat(currentInput.length));
                    currentInput = prev;
                    term.write(prev);
                }
            }
            return;
        }

        if (ev.key === 'ArrowDown') {
            if (historyIndex < history.length - 1) {
                historyIndex++;
                const next = history[historyIndex];
                if (next !== undefined) {
                    term.write('\b \b'.repeat(currentInput.length));
                    currentInput = next;
                    term.write(next);
                }
            } else {
                historyIndex = history.length;
                term.write('\b \b'.repeat(currentInput.length));
                currentInput = '';
            }
            return;
        }

        // Printable characters
        if (key.length === 1 && key.charCodeAt(0) >= 32) {
            currentInput += key;
            term.write(key);
        }
    });

    // Initial prompt
    term.writeln('Welcome to Tuxpedia Terminal Simulator!');
    term.writeln('Type `help` for a list of commands.');
    writePrompt();

    // Auto-resize
    window.addEventListener('resize', () => {
        fitAddon.fit();
    });
}

// Export for use in Livewire
window.initTerminal = initTerminal;
