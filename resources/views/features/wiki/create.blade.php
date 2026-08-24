@extends('layouts.app')

@section('title', 'Create Wiki Page')

@section('content')
<div class="container mx-auto px-4 py-8">
    <h1 class="text-3xl font-bold mb-6">Create New Page</h1>
    <livewire:wiki-editor />
</div>
@endsection
