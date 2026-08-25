<?php

use Illuminate\Support\Facades\Route;
use App\Features\DistroComparison\Http\Livewire\ComparisonTable;

Route::get('/distro-comparison', function () {
    return view('features.distro-comparison.index');
})->name('distro-comparison.index');
