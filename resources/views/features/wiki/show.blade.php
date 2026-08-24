@extends('layouts.app')

@section('title', $page->title)

@section('content')
<div class="container mx-auto px-4 py-8">
    <div class="flex justify-between items-start mb-6">
        <div>
            <h1 class="text-4xl font-bold">{{ $page->title }}</h1>
            <p class="text-gray-500 text-sm">Slug: {{ $page->slug }}</p>
            @if($page->author)
                <p class="text-gray-500 text-sm">Author: {{ $page->author }}</p>
            @endif
            <p class="text-gray-400 text-xs">Last updated: {{ $page->updated_at->format('Y-m-d H:i') }}</p>
        </div>
        <div class="space-x-2">
            <a href="{{ route('wiki.edit', $page->slug) }}" class="bg-yellow-500 hover:bg-yellow-700 text-white font-bold py-2 px-4 rounded">
                Edit
            </a>
        </div>
    </div>

    <div class="prose max-w-none mt-8">
        {!! \Illuminate\Support\Str::markdown($page->content) !!}
    </div>
</div>
@endsections
