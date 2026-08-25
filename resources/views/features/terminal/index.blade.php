@extends('layouts.app')

@section('title', 'Terminal Simulator')

@section('content')
<div class="container mx-auto px-4 py-8">
    <h1 class="text-3xl font-bold mb-6">🐧 Terminal Simulator</h1>
    <p class="text-gray-600 mb-4">Experience a Linux terminal right in your browser. Try common commands like <code>ls</code>, <code>pwd</code>, <code>echo</code>, and more.</p>

    <livewire:terminal />
</div>
@endsection
