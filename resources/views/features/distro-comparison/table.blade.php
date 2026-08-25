<div>
    <!-- Search & Filters -->
    <div class="mb-6 flex flex-wrap gap-4 items-end">
        <div class="flex-1 min-w-[200px]">
            <label class="block text-sm font-medium text-gray-700 mb-1">Search</label>
            <input wire:model.live="search" type="text" placeholder="Search distros..." class="w-full border rounded px-3 py-2">
        </div>
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Based On</label>
            <select wire:model.live="filters.based_on" class="border rounded px-3 py-2">
                <option value="">All</option>
                @foreach($basedOnOptions as $option)
                    <option value="{{ $option }}">{{ $option }}</option>
                @endforeach
            </select>
        </div>
        <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Release Model</label>
            <select wire:model.live="filters.release_model" class="border rounded px-3 py-2">
                <option value="">All</option>
                @foreach($releaseModelOptions as $option)
                    <option value="{{ $option }}">{{ $option }}</option>
                @endforeach
            </select>
        </div>
        <div>
            <button wire:click="resetFilters" class="bg-gray-200 hover:bg-gray-300 px-4 py-2 rounded">Reset</button>
        </div>
    </div>

    <!-- Table -->
    <div class="overflow-x-auto">
        <table class="min-w-full bg-white border">
            <thead>
                <tr class="bg-gray-100">
                    <th wire:click="sortBy('name')" class="px-4 py-2 text-left cursor-pointer hover:bg-gray-200">Name</th>
                    <th wire:click="sortBy('based_on')" class="px-4 py-2 text-left cursor-pointer hover:bg-gray-200">Based On</th>
                    <th wire:click="sortBy('package_manager')" class="px-4 py-2 text-left cursor-pointer hover:bg-gray-200">Package Manager</th>
                    <th wire:click="sortBy('default_desktop')" class="px-4 py-2 text-left cursor-pointer hover:bg-gray-200">Default Desktop</th>
                    <th wire:click="sortBy('release_model')" class="px-4 py-2 text-left cursor-pointer hover:bg-gray-200">Release Model</th>
                    <th wire:click="sortBy('architecture')" class="px-4 py-2 text-left cursor-pointer hover:bg-gray-200">Architecture</th>
                    <th class="px-4 py-2 text-left">Description</th>
                </tr>
            </thead>
            <tbody>
                @forelse($distributions as $distro)
                    <tr class="border-t hover:bg-gray-50">
                        <td class="px-4 py-2 font-medium">{{ $distro->name }}</td>
                        <td class="px-4 py-2">{{ $distro->based_on ?? '—' }}</td>
                        <td class="px-4 py-2">{{ $distro->package_manager ?? '—' }}</td>
                        <td class="px-4 py-2">{{ $distro->default_desktop ?? '—' }}</td>
                        <td class="px-4 py-2">{{ $distro->release_model ?? '—' }}</td>
                        <td class="px-4 py-2">{{ $distro->architecture ?? '—' }}</td>
                        <td class="px-4 py-2 text-sm">{{ Str::limit($distro->description, 60) }}</td>
                    </tr>
                @empty
                    <tr>
                        <td colspan="7" class="px-4 py-4 text-center text-gray-500">No distributions found.</td>
                    </tr>
                @endforelse
            </tbody>
        </table>
    </div>

    <!-- Pagination -->
    <div class="mt-4">
        {{ $distributions->links() }}
    </div>
</div>
