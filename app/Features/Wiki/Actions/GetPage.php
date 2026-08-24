<?php

declare(strict_types=1);

namespace App\Features\Wiki\Actions;

use App\Features\Wiki\Models\Page;
use App\Features\Wiki\Exceptions\PageNotFoundException;

final class GetPage
{
    public function execute(string $slug): Page
    {
        $page = Page::where('slug', $slug)->first();
        if (!$page) {
            throw new PageNotFoundException($slug);
        }
        return $page;
    }
}
