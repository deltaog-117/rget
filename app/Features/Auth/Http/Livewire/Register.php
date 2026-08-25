<?php

declare(strict_types=1);

namespace App\Features\Auth\Http\Livewire;

use App\Features\Auth\Actions\RegisterUser;
use Livewire\Component;

class Register extends Component
{
    public string $name = '';
    public string $email = '';
    public string $password = '';
    public string $password_confirmation = '';
    public string $error = '';

    protected $rules = [
        'name' => 'required|string|max:255',
        'email' => 'required|email|unique:users,email',
        'password' => 'required|min:8|confirmed',
    ];

    public function register(RegisterUser $registerUser)
    {
        $this->validate();
        $this->error = '';

        try {
            $user = $registerUser->execute($this->name, $this->email, $this->password);
            auth()->login($user);
            return redirect()->route('wiki.index')->with('success', 'Registration successful!');
        } catch (\Illuminate\Validation\ValidationException $e) {
            $this->error = $e->errors()['email'][0] ?? $e->getMessage();
        } catch (\Exception $e) {
            $this->error = 'An error occurred: ' . $e->getMessage();
        }
    }

    public function render()
    {
        return view('features.auth.register');
    }
}
