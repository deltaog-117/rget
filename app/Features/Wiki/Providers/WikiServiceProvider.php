<?php

declare(strict_types=1);

namespace App\Features\Wiki\Providers;

use Illuminate\Support\ServiceProvider;
use Illuminate\Support\Facades\Route;
use Livewire\Livewire;
use App\Features\Wiki\Http\Livewire\WikiEditor;

class WikiServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        // Bind actions
        $this->app->bind(\App\Features\Wiki\Actions\CreatePage::class);
        $this->app->bind(\App\Features\Wiki\Actions\UpdatePage::class);
        $this->app->bind(\App\Features\Wiki\Actions\DeletePage::class);
        $this->app->bind(\App\Features\Wiki\Actions\GetPage::class);
    }

    public function boot(): void
    {
        // Register Livewire components
        Livewire::component('wiki-editor', WikiEditor::class);

        // Load routes
        Route::group([], base_path('app/Features/Wiki/routes.php'));

        // Load migrations
        $this->loadMigrationsFrom([
            base_path('app/Features/Wiki/Database/Migrations'),
        ]);
    }
}
