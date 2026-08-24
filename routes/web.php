<?php

use Illuminate\Support\Facades\Route;

// Load each feature's routes automatically
foreach (glob(app_path('Features/*/routes.php')) as $routeFile) {
    require $routeFile;
}

// Default welcome page (optional)
Route::get('/', function () {
    return view('welcome');
});
