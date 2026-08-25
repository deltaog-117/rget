@extends('layouts.app')

@section('title', 'All Wiki Pages')

@section('content')
<div class="container mx-auto px-4 py-8">
    <div class="flex justify-between items-center mb-6">
        <h1 class="text-3xl font-bold">Wiki Pages</h1>
        <a href="{{ route('wiki.create') }}" class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">
            + New Page
        </a>
    </div>

    @if($pages->isEmpty())
        <p class="text-gray-600">No pages yet. Be the first to create one!</p>
    @else
        <ul class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            @foreach($pages as $page)
                <li class="border rounded p-4 shadow hover:shadow-lg transition">
                    <a href="{{ route('wiki.show', $page->slug) }}" class="block">
                        <div class="flex items-start justify-between">
                            <h2 class="text-xl font-semibold text-blue-600 hover:underline">{{ $page->title }}</h2>
                            @if($page->wasUpdatedSinceLastVisit(auth()->user()))
                                <span class="bg-green-100 text-green-800 text-xs font-medium px-2 py-0.5 rounded">New</span>
                            @endif
                        </div>
                        <p class="text-sm text-gray-500 mt-1">Slug: {{ $page->slug }}</p>
                        <p class="text-sm text-gray-500">Updated: {{ $page->updated_at->diffForHumans() }}</p>
                    </a>
                </li>
            @endforeach
        </ul>
    @endif
</div>
@endsection
