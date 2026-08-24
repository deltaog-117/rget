<?php

declare(strict_types=1);

namespace App\Features\Wiki\Http\Livewire;

use App\Features\Wiki\Actions\CreatePage;
use App\Features\Wiki\Actions\UpdatePage;
use App\Features\Wiki\Actions\GetPage;
use App\Features\Wiki\Exceptions\InvalidSlugException;
use App\Features\Wiki\Exceptions\PageNotFoundException;
use App\Features\Wiki\Models\Page;
use App\Features\Wiki\ValueObjects\Content;
use App\Features\Wiki\ValueObjects\Slug;
use App\Features\Wiki\ValueObjects\Title;
use Livewire\Component;
use Illuminate\Support\Facades\Log;

class WikiEditor extends Component
{
    public ?Page $page = null;

    public string $slug = '';
    public string $title = '';
    public string $content = '';
    public string $author = '';

    public bool $isEditing = false;
    public string $errorMessage = '';

    protected $rules = [
        'slug' => 'required|string|max:255|regex:/^[a-z0-9\-_]+$/',
        'title' => 'required|string|max:255',
        'content' => 'nullable|string',
        'author' => 'nullable|string|max:255',
    ];

    public function mount(?string $slug = null): void
    {
        if ($slug) {
            try {
                $getPage = app(GetPage::class);
                $this->page = $getPage->execute($slug);
                $this->slug = $this->page->slug->toString();
                $this->title = $this->page->title->toString();
                $this->content = $this->page->content->toString();
                $this->author = $this->page->author ?? '';
                $this->isEditing = true;
            } catch (PageNotFoundException $e) {
                abort(404, $e->getMessage());
            }
        } else {
            $this->isEditing = false;
        }
    }

    public function save()
    {
        $this->validate();

        try {
            $slugObj = Slug::fromString($this->slug);
            $titleObj = Title::fromString($this->title);
            $contentObj = Content::fromString($this->content);
            $author = $this->author ?: null;

            if ($this->isEditing) {
                $updatePage = app(UpdatePage::class);
                $this->page = $updatePage->execute($this->page, $slugObj, $titleObj, $contentObj, $author);
                session()->flash('message', 'Page updated successfully.');
                Log::info('Wiki page edited via Livewire', ['slug' => $slugObj->toString()]);
            } else {
                $createPage = app(CreatePage::class);
                $this->page = $createPage->execute($slugObj, $titleObj, $contentObj, $author);
                session()->flash('message', 'Page created successfully.');
                Log::info('Wiki page created via Livewire', ['slug' => $slugObj->toString()]);
            }

            return redirect()->route('wiki.show', $this->page->slug->toString());
        } catch (InvalidSlugException $e) {
            $this->errorMessage = $e->getMessage();
        } catch (\InvalidArgumentException $e) {
            $this->errorMessage = $e->getMessage();
        } catch (\RuntimeException $e) {
            $this->errorMessage = $e->getMessage();
        }
    }

    public function render()
    {
        return view('features.wiki.editor')
            ->layout('layouts.app');
    }
}
