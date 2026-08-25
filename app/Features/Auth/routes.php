<?php

use Illuminate\Support\Facades\Route;
use App\Features\Auth\Actions\LogoutUser;

Route::get('/login', function () {
    return view('features.auth.login-wrapper');
})->name('auth.login');

Route::get('/register', function () {
    return view('features.auth.register-wrapper');
})->name('auth.register');

Route::post('/logout', function (LogoutUser $logout) {
    $logout->execute();
    return redirect()->route('auth.login');
})->name('auth.logout');
