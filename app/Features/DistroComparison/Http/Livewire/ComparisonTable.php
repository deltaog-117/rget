<?php

declare(strict_types=1);

namespace App\Features\DistroComparison\Http\Livewire;

use App\Features\DistroComparison\Models\Distribution;
use Livewire\Component;
use Livewire\WithPagination;

class ComparisonTable extends Component
{
    use WithPagination;

    public string $search = '';
    public string $sortField = 'name';
    public string $sortDirection = 'asc';
    public array $filters = [
        'based_on' => '',
        'release_model' => '',
    ];

    protected $queryString = [
        'search' => ['except' => ''],
        'sortField' => ['except' => 'name'],
        'sortDirection' => ['except' => 'asc'],
        'filters' => ['except' => ['based_on' => '', 'release_model' => '']],
    ];

    public function sortBy(string $field): void
    {
        if ($this->sortField === $field) {
            $this->sortDirection = $this->sortDirection === 'asc' ? 'desc' : 'asc';
        } else {
            $this->sortField = $field;
            $this->sortDirection = 'asc';
        }
    }

    public function resetFilters(): void
    {
        $this->reset(['search', 'filters']);
        $this->resetPage();
    }

    public function updatingSearch(): void
    {
        $this->resetPage();
    }

    public function render()
    {
        $query = Distribution::query();

        if (!empty($this->search)) {
            $search = '%' . $this->search . '%';
            $query->where(function ($q) use ($search) {
                $q->where('name', 'like', $search)
                  ->orWhere('description', 'like', $search)
                  ->orWhere('based_on', 'like', $search)
                  ->orWhere('package_manager', 'like', $search);
            });
        }

        if (!empty($this->filters['based_on'])) {
            $query->where('based_on', $this->filters['based_on']);
        }
        if (!empty($this->filters['release_model'])) {
            $query->where('release_model', 'like', '%' . $this->filters['release_model'] . '%');
        }

        $query->orderBy($this->sortField, $this->sortDirection);
        $distributions = $query->paginate(10);

        $basedOnOptions = Distribution::distinct()->pluck('based_on')->filter()->values();
        $releaseModelOptions = Distribution::distinct()->pluck('release_model')->filter()->values();

        return view('features.distro-comparison.table', [
            'distributions' => $distributions,
            'basedOnOptions' => $basedOnOptions,
            'releaseModelOptions' => $releaseModelOptions,
        ]);
    }
}
