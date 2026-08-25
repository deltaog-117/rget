@extends('layouts.app')

@section('title', 'Search Results')

@section('content')
<div class="container mx-auto px-4 py-8">
    <h1 class="text-3xl font-bold mb-6">Search Results</h1>
    <livewire:search-results :query="request('q')" />
</div>
@endsection
