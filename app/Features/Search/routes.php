<?php

use Illuminate\Support\Facades\Route;

Route::get('/search', function () {
    return view('features.search.page');
})->name('search.results');
