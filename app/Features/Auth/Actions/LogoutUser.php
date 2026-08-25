<?php

declare(strict_types=1);

namespace App\Features\Auth\Actions;

use Illuminate\Support\Facades\Auth;

class LogoutUser
{
    public function execute(): void
    {
        Auth::logout();
    }
}
