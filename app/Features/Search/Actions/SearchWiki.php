<?php

declare(strict_types=1);

namespace App\Features\Search\Actions;

use App\Features\Wiki\Models\Page;
use Illuminate\Support\Facades\DB;

class SearchWiki
{
    public function execute(string $query, int $perPage = 10)
    {
        if (empty(trim($query))) {
            return Page::orderBy('title')->paginate($perPage);
        }

        $results = Page::selectRaw(
            'wiki_pages.*, MATCH(title, content) AGAINST(?) AS relevance',
            [$query]
        )
        ->whereRaw('MATCH(title, content) AGAINST(?)', [$query])
        ->orderByDesc('relevance')
        ->paginate($perPage)
        ->appends(['q' => $query]);

        return $results;
    }
}
