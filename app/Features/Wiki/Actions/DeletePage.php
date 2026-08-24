<?php

declare(strict_types=1);

namespace App\Features\Wiki\Actions;

use App\Features\Wiki\Models\Page;
use App\Shared\Services\GitService;
use Illuminate\Support\Facades\Log;

final class DeletePage
{
    public function __construct(
        private GitService $git,
    ) {}

    public function execute(Page $page): void
    {
        $slug = $page->slug->toString();

        $this->git->deleteFile(
            $page->getMarkdownFilePath(),
            "Delete page: {$slug}"
        );

        $page->delete();

        Log::info('Wiki page deleted', [
            'slug' => $slug,
            'page_id' => $page->id,
        ]);
    }
}
