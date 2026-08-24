<?php

use Illuminate\Support\Facades\Route;
use App\Features\Wiki\Http\Controllers\PageController;

Route::get('/wiki', [PageController::class, 'index'])->name('wiki.index');
Route::get('/wiki/create', function () {
    return view('features.wiki.create');
})->name('wiki.create');
Route::get('/wiki/{slug}', [PageController::class, 'show'])->name('wiki.show');
Route::get('/wiki/{slug}/edit', function ($slug) {
    return view('features.wiki.edit', ['slug' => $slug]);
})->name('wiki.edit');
