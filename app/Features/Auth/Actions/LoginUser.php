<?php

declare(strict_types=1);

namespace App\Features\Auth\Actions;

use App\Features\Auth\Models\User;
use Illuminate\Support\Facades\Auth;
use Illuminate\Validation\ValidationException;

class LoginUser
{
    public function execute(string $email, string $password, bool $remember = false): User
    {
        if (!Auth::attempt(['email' => $email, 'password' => $password], $remember)) {
            throw ValidationException::withMessages([
                'email' => __('auth.failed'),
            ]);
        }

        /** @var User $user */
        $user = Auth::user();
        return $user;
    }
}
