<?php

declare(strict_types=1);

namespace App\Features\Auth\Providers;

use Illuminate\Support\ServiceProvider;
use Illuminate\Support\Facades\Route;
use Livewire\Livewire;
use App\Features\Auth\Http\Livewire\Login;
use App\Features\Auth\Http\Livewire\Register;

class AuthServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->app->bind(\App\Features\Auth\Actions\LoginUser::class);
        $this->app->bind(\App\Features\Auth\Actions\RegisterUser::class);
        $this->app->bind(\App\Features\Auth\Actions\LogoutUser::class);
    }

    public function boot(): void
    {
        Livewire::component('auth-login', Login::class);
        Livewire::component('auth-register', Register::class);

        Route::group([], base_path('app/Features/Auth/routes.php'));
    }
}
