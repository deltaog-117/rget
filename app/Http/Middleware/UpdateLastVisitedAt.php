<?php

namespace App\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Auth;
use Carbon\Carbon;

class UpdateLastVisitedAt
{
    public function handle(Request $request, Closure $next)
    {
        if (Auth::check()) {
            $user = Auth::user();
            // Ensure last_visited_at is a Carbon instance
            $lastVisited = $user->last_visited_at ? Carbon::parse($user->last_visited_at) : null;
            if (!$lastVisited || $lastVisited->diffInMinutes(now()) >= 5) {
                $user->last_visited_at = now();
                $user->save();
            }
        }

        return $next($request);
    }
}
