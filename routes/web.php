<?php

use Illuminate\Support\Facades\Route;

foreach (glob(app_path('Features/*/routes.php')) as $routeFile) {
    require $routeFile;
}

Route::get('/', function () {
    return redirect()->route('wiki.index');
});
