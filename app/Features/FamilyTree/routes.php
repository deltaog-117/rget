<?php

use Illuminate\Support\Facades\Route;

Route::get('/family-tree', function () {
    return view('features.family-tree.index');
})->name('family-tree.index');
