<?php

use Illuminate\Support\Facades\Route;

Route::get('/terminal', function () {
    return view('features.terminal.index');
})->name('terminal.index');
