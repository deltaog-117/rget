<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="X-UA-Compatible" content="ie=edge">
    <title>@yield('title', 'Tuxpedia')</title>
    <link rel="stylesheet" href="{{ asset('css/app.css') }}">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:opsz,wght@14..32,400;14..32,600;14..32,700&family=JetBrains+Mono&display=swap" rel="stylesheet">
    @livewireStyles
</head>
<body class="min-h-screen flex flex-col">
    <nav class="bg-tux-surface border-b border-tux-border sticky top-0 z-50">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
            <div class="flex justify-between items-center h-16">
                <div class="flex items-center space-x-6">
                    <a href="{{ route('wiki.index') }}" class="text-xl font-bold text-tux-text flex items-center gap-2">
                        <span>🐧</span> Tuxpedia
                    </a>
                    <div class="hidden md:flex space-x-4">
                        <a href="{{ route('wiki.index') }}" class="text-tux-muted hover:text-tux-text transition">Wiki</a>
                        <a href="{{ route('wiki.create') }}" class="text-tux-muted hover:text-tux-text transition">New Page</a>
                        <a href="{{ route('distro-comparison.index') }}" class="text-tux-muted hover:text-tux-text transition">Distros</a>
                        <a href="{{ route('family-tree.index') }}" class="text-tux-muted hover:text-tux-text transition">Family Tree</a>
                        <a href="{{ route('terminal.index') }}" class="text-tux-muted hover:text-tux-text transition">Terminal</a>
                    </div>
                </div>
                <div class="flex items-center space-x-4">
                    @auth
                        <span class="text-sm text-tux-muted hidden sm:inline">{{ auth()->user()->name }}</span>
                        <form action="{{ route('auth.logout') }}" method="POST" class="inline">
                            @csrf
                            <button type="submit" class="text-sm text-tux-muted hover:text-tux-text transition">Logout</button>
                        </form>
                    @else
                        <a href="{{ route('auth.login') }}" class="text-sm text-tux-muted hover:text-tux-text transition">Login</a>
                        <a href="{{ route('auth.register') }}" class="text-sm text-tux-muted hover:text-tux-text transition">Register</a>
                    @endauth
                    <div class="w-48">
                        <livewire:search-bar />
                    </div>
                </div>
            </div>
        </div>
    </nav>

    <main class="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-6">
        @yield('content')
    </main>

    <footer class="border-t border-tux-border bg-tux-surface mt-8">
        <div class="max-w-7xl mx-auto px-4 py-4 text-sm text-tux-muted text-center">
            Tuxpedia – the Linux encyclopedia. Built with ❤️ and Laravel.
        </div>
    </footer>

    @livewireScripts
    @stack('scripts')
</body>
</html>
