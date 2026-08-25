<div>
    <div id="terminal-container" style="width: 100%; height: 500px; background: #1e1e1e; border-radius: 0.5rem; overflow: hidden;"></div>

    <!-- Load xterm.js from CDN -->
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/xterm/css/xterm.css" />
    <script src="https://cdn.jsdelivr.net/npm/xterm/lib/xterm.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/xterm-addon-fit/lib/xterm-addon-fit.js"></script>

    <!-- Our terminal simulator script -->
    <script src="{{ asset('js/terminal-simulator.js') }}"></script>

    <script>
        document.addEventListener('livewire:init', function () {
            if (typeof initTerminal === 'function') {
                initTerminal('terminal-container');
            } else {
                console.warn('Terminal simulator not loaded.');
            }
        });

        // Also run on DOM ready for direct page load
        document.addEventListener('DOMContentLoaded', function () {
            if (typeof initTerminal === 'function') {
                // Wait a bit for the container to render
                setTimeout(function () {
                    initTerminal('terminal-container');
                }, 300);
            }
        });
    </script>

    <div class="mt-4 text-sm text-gray-500">
        <p>Try commands: <code class="bg-gray-100 px-2 py-1 rounded">ls</code>, <code class="bg-gray-100 px-2 py-1 rounded">pwd</code>, <code class="bg-gray-100 px-2 py-1 rounded">echo hello</code>, <code class="bg-gray-100 px-2 py-1 rounded">cat /etc/os-release</code></p>
    </div>
</div>
