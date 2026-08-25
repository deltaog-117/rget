<?php

declare(strict_types=1);

namespace App\Features\Auth\Http\Livewire;

use App\Features\Auth\Actions\LoginUser;
use Livewire\Component;
use Illuminate\Validation\ValidationException;

class Login extends Component
{
    public string $email = '';
    public string $password = '';
    public bool $remember = false;
    public string $error = '';

    protected $rules = [
        'email' => 'required|email',
        'password' => 'required|min:8',
    ];

    public function login(LoginUser $loginUser)
    {
        $this->validate();
        $this->error = '';

        try {
            $user = $loginUser->execute($this->email, $this->password, $this->remember);
            return redirect()->route('wiki.index')->with('success', 'Welcome back!');
        } catch (ValidationException $e) {
            $this->error = $e->errors()['email'][0] ?? 'Invalid credentials.';
        } catch (\Exception $e) {
            $this->error = 'An error occurred. Please try again.';
        }
    }

    public function render()
    {
        return view('features.auth.login');
    }
}
