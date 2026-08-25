<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="X-UA-Compatible" content="ie=edge">
    <title>@yield('title', 'Tuxpedia')</title>
    @livewireStyles
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: system-ui, -apple-system, sans-serif; background: #f7fafc; }
        nav { background: #ffffff; border-bottom: 1px solid #e2e8f0; padding: 0.75rem 1.5rem; }
        .container { max-width: 1200px; margin: 0 auto; padding: 0 1rem; }
        .flex { display: flex; }
        .justify-between { justify-content: space-between; }
        .items-center { align-items: center; }
        .space-x-4 > * + * { margin-left: 1rem; }
        .text-xl { font-size: 1.25rem; }
        .font-bold { font-weight: 700; }
        .text-gray-800 { color: #2d3748; }
        .text-gray-700 { color: #4a5568; }
        .hover\:text-gray-900:hover { color: #1a202c; }
        .px-4 { padding-left: 1rem; padding-right: 1rem; }
        .py-8 { padding-top: 2rem; padding-bottom: 2rem; }
        .bg-white { background: #ffffff; }
        .shadow { box-shadow: 0 1px 3px 0 rgba(0,0,0,0.1); }
        .rounded { border-radius: 0.25rem; }
        .border { border: 1px solid #e2e8f0; }
        .p-4 { padding: 1rem; }
        .mt-4 { margin-top: 1rem; }
        .mb-6 { margin-bottom: 1.5rem; }
        .grid { display: grid; }
        .grid-cols-1 { grid-template-columns: repeat(1, minmax(0, 1fr)); }
        .gap-4 { gap: 1rem; }
        .text-3xl { font-size: 1.875rem; }
        .text-blue-600 { color: #3182ce; }
        .hover\:underline:hover { text-decoration: underline; }
        .bg-blue-500 { background-color: #4299e1; }
        .hover\:bg-blue-700:hover { background-color: #2b6cb0; }
        .text-white { color: #ffffff; }
        .py-2 { padding-top: 0.5rem; padding-bottom: 0.5rem; }
        .inline-block { display: inline-block; }
        .mt-8 { margin-top: 2rem; }
        .prose { max-width: 65ch; }
        .max-w-none { max-width: none; }
        .text-gray-500 { color: #718096; }
        .text-sm { font-size: 0.875rem; }
        .text-xs { font-size: 0.75rem; }
        .italic { font-style: italic; }
        .font-mono { font-family: monospace; }
        .border-red-500 { border-color: #f56565; }
        .text-red-500 { color: #f56565; }
        .bg-red-100 { background-color: #fff5f5; }
        .border-red-400 { border-color: #fc8181; }
        .text-red-700 { color: #c53030; }
        .bg-green-100 { background-color: #f0fff4; }
        .border-green-400 { border-color: #68d391; }
        .text-green-700 { color: #276749; }
        .w-full { width: 100%; }
        .block { display: block; }
        .mb-2 { margin-bottom: 0.5rem; }
        .mb-4 { margin-bottom: 1rem; }
        .shadow-appearance { box-shadow: 0 1px 3px 0 rgba(0,0,0,0.1), 0 1px 2px 0 rgba(0,0,0,0.06); }
        .max-w-md { max-width: 28rem; }
        .mx-auto { margin-left: auto; margin-right: auto; }
        .text-center { text-align: center; }
        .text-2xl { font-size: 1.5rem; }
        @media (min-width: 640px) { .sm\:flex { display: flex; } }
        @media (min-width: 768px) { .md\:grid-cols-2 { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
        @media (min-width: 1024px) { .lg\:grid-cols-3 { grid-template-columns: repeat(3, minmax(0, 1fr)); } }
    </style>
</head>
<body>
    <nav>
        <div class="container flex justify-between items-center">
            <div class="flex items-center space-x-4">
                <a href="{{ route('wiki.index') }}" class="text-xl font-bold text-gray-800">Tuxpedia</a>
                <a href="{{ route('wiki.index') }}" class="text-gray-700 hover:text-gray-900">Wiki</a>
                <a href="{{ route('wiki.create') }}" class="text-gray-700 hover:text-gray-900">New Page</a>
                <a href="{{ route('distro-comparison.index') }}" class="text-gray-700 hover:text-gray-900">Distros</a>
                <a href="{{ route('family-tree.index') }}" class="text-gray-700 hover:text-gray-900">Family Tree</a>
            </div>
            <div class="flex items-center space-x-4">
                @auth
                    <span class="text-sm text-gray-600">{{ auth()->user()->name }}</span>
                    <form action="{{ route('auth.logout') }}" method="POST" class="inline">
                        @csrf
                        <button type="submit" class="text-gray-700 hover:text-gray-900">Logout</button>
                    </form>
                @else
                    <a href="{{ route('auth.login') }}" class="text-gray-700 hover:text-gray-900">Login</a>
                    <a href="{{ route('auth.register') }}" class="text-gray-700 hover:text-gray-900">Register</a>
                @endauth
                <livewire:search-bar />
            </div>
        </div>
    </nav>

    <div class="container px-4 py-8">
        @yield('content')
    </div>

    @livewireScripts
</body>
</html>
