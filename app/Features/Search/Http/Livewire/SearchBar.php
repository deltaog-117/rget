<?php

declare(strict_types=1);

namespace App\Features\Search\Http\Livewire;

use Livewire\Component;

class SearchBar extends Component
{
    public string $query = '';

    protected $queryString = ['query' => ['except' => '']];

    public function search()
    {
        if (trim($this->query) !== '') {
            return redirect()->route('search.results', ['q' => $this->query]);
        }
    }

    public function render()
    {
        return view('features.search.bar');
    }
}
