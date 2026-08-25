<div class="max-w-md mx-auto bg-white p-6 rounded shadow">
    <h2 class="text-2xl font-bold mb-6 text-center">Register</h2>

    <form wire:submit.prevent="register">
        <div class="mb-4">
            <label class="block text-gray-700 text-sm font-bold mb-2">Name</label>
            <input wire:model="name" type="text" class="w-full border rounded px-3 py-2" required>
            @error('name') <p class="text-red-500 text-xs mt-1">{{ $message }}</p> @enderror
        </div>

        <div class="mb-4">
            <label class="block text-gray-700 text-sm font-bold mb-2">Email</label>
            <input wire:model="email" type="email" class="w-full border rounded px-3 py-2" required>
            @error('email') <p class="text-red-500 text-xs mt-1">{{ $message }}</p> @enderror
        </div>

        <div class="mb-4">
            <label class="block text-gray-700 text-sm font-bold mb-2">Password</label>
            <input wire:model="password" type="password" class="w-full border rounded px-3 py-2" required>
            @error('password') <p class="text-red-500 text-xs mt-1">{{ $message }}</p> @enderror
        </div>

        <div class="mb-4">
            <label class="block text-gray-700 text-sm font-bold mb-2">Confirm Password</label>
            <input wire:model="password_confirmation" type="password" class="w-full border rounded px-3 py-2" required>
        </div>

        <button type="submit" class="w-full bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">
            Register
        </button>
    </form>

    <p class="mt-4 text-center text-sm text-gray-600">
        Already have an account? <a href="{{ route('auth.login') }}" class="text-blue-500 hover:underline">Login</a>
    </p>
</div>
