<?php

declare(strict_types=1);

namespace App\Features\Search\Http\Livewire;

use App\Features\Search\Actions\SearchWiki;
use Livewire\Component;

class SearchResults extends Component
{
    public string $query = '';

    public function mount(string $query = ''): void
    {
        $this->query = $query;
    }

    public function render()
    {
        $results = null;

        if (!empty(trim($this->query))) {
            $search = app(SearchWiki::class);
            $results = $search->execute($this->query, 10);
        }

        return view('features.search.results', [
            'results' => $results,
            'query' => $this->query,
        ]);
    }
}
