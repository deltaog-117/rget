<?php

declare(strict_types=1);

namespace App\Features\Search\Providers;

use Illuminate\Support\ServiceProvider;
use Illuminate\Support\Facades\Route;
use Livewire\Livewire;
use App\Features\Search\Http\Livewire\SearchBar;
use App\Features\Search\Http\Livewire\SearchResults;

class SearchServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->app->bind(\App\Features\Search\Actions\SearchWiki::class);
    }

    public function boot(): void
    {
        Livewire::component('search-bar', SearchBar::class);
        Livewire::component('search-results', SearchResults::class);

        Route::group([], base_path('app/Features/Search/routes.php'));
    }
}
