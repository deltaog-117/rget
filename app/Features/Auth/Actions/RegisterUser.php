<?php

declare(strict_types=1);

namespace App\Features\Auth\Actions;

use App\Features\Auth\Models\User;
use Illuminate\Support\Facades\Hash;

class RegisterUser
{
    public function execute(string $name, string $email, string $password): User
    {
        $user = User::create([
            'name' => $name,
            'email' => $email,
            'password' => Hash::make($password),
        ]);

        return $user;
    }
}
