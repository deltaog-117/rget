<?php

declare(strict_types=1);

namespace App\Features\Terminal\Providers;

use Illuminate\Support\ServiceProvider;
use Illuminate\Support\Facades\Route;
use Livewire\Livewire;
use App\Features\Terminal\Http\Livewire\Terminal;

class TerminalServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        //
    }

    public function boot(): void
    {
        Livewire::component('terminal', Terminal::class);

        Route::group([], base_path('app/Features/Terminal/routes.php'));
    }
}
