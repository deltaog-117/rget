@extends('layouts.app')

@section('title', 'Distro Comparison')

@section('content')
<div class="container mx-auto px-4 py-8">
    <h1 class="text-3xl font-bold mb-6">Linux Distribution Comparison</h1>

    @livewire('comparison-table')
</div>
@endsection
