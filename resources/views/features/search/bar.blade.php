<form wire:submit.prevent="search" class="flex items-center">
    <input wire:model="query" type="search" placeholder="Search wiki..." class="border rounded-l px-3 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" aria-label="Search">
    <button type="submit" class="bg-blue-500 hover:bg-blue-700 text-white px-3 py-1 rounded-r text-sm">Go</button>
</form>
