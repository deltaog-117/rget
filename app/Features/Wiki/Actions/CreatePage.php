<?php

declare(strict_types=1);

namespace App\Features\Wiki\Actions;

use App\Features\Wiki\Models\Page;
use App\Features\Wiki\ValueObjects\Content;
use App\Features\Wiki\ValueObjects\Slug;
use App\Features\Wiki\ValueObjects\Title;
use App\Shared\Services\GitService;
use Illuminate\Support\Facades\Log;

final class CreatePage
{
    public function __construct(
        private GitService $git,
    ) {}

    public function execute(Slug $slug, Title $title, Content $content, ?string $author = null): Page
    {
        if (Page::where('slug', $slug->toString())->exists()) {
            throw new \RuntimeException("Page with slug '{$slug}' already exists.");
        }

        $page = new Page();
        $page->slug = $slug;
        $page->title = $title;
        $page->content = $content;
        $page->author = $author;
        $page->save();

        $this->git->commitFile(
            $page->getMarkdownFilePath(),
            $content->toString(),
            "Create page: {$page->slug}"
        );

        Log::info('Wiki page created', [
            'slug' => $slug->toString(),
            'author' => $author,
            'page_id' => $page->id,
        ]);

        return $page;
    }
}
