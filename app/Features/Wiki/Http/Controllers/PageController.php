<?php

declare(strict_types=1);

namespace App\Features\Wiki\Http\Controllers;

use App\Features\Wiki\Actions\GetPage;
use App\Features\Wiki\Exceptions\PageNotFoundException;
use Illuminate\Http\Request;
use Illuminate\View\View;

class PageController
{
    public function show(Request $request, string $slug, GetPage $getPage): View
    {
        try {
            $page = $getPage->execute($slug);
            return view('features.wiki.show', ['page' => $page]);
        } catch (PageNotFoundException $e) {
            abort(404, $e->getMessage());
        }
    }

    public function index(): View
    {
        $pages = \App\Features\Wiki\Models\Page::orderBy('title')->get();
        return view('features.wiki.index', ['pages' => $pages]);
    }
}
