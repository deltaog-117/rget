<?php

declare(strict_types=1);

namespace App\Features\DistroComparison\Providers;

use Illuminate\Support\ServiceProvider;
use Illuminate\Support\Facades\Route;
use Livewire\Livewire;
use App\Features\DistroComparison\Http\Livewire\ComparisonTable;

class DistroComparisonServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        //
    }

    public function boot(): void
    {
        Livewire::component('comparison-table', ComparisonTable::class);

        Route::group([], base_path('app/Features/DistroComparison/routes.php'));

        $this->loadMigrationsFrom([
            base_path('app/Features/DistroComparison/Database/Migrations'),
        ]);
    }
}
