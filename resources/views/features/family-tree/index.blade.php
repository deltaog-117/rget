@extends('layouts.app')

@section('title', 'Distro Family Tree')

@section('content')
<div class="container mx-auto px-4 py-8">
    <h1 class="text-3xl font-bold mb-6">Linux Distribution Family Tree</h1>
    <p class="text-gray-600 mb-4">Interactive tree showing the lineage of major Linux distributions.</p>

    <livewire:family-tree />
</div>
@endsection
