<?php

declare(strict_types=1);

namespace App\Features\Wiki\Models;

use App\Features\Wiki\ValueObjects\Content;
use App\Features\Wiki\ValueObjects\Slug;
use App\Features\Wiki\ValueObjects\Title;
use Illuminate\Database\Eloquent\Model;

class Page extends Model
{
    protected $table = 'wiki_pages';

    protected $fillable = [
        'slug',
        'title',
        'content',
        'author',
    ];

    protected $casts = [
        'created_at' => 'datetime',
        'updated_at' => 'datetime',
    ];

    public function getSlugAttribute(string $value): Slug
    {
        return Slug::fromString($value);
    }

    public function setSlugAttribute(Slug|string $value): void
    {
        $this->attributes['slug'] = (string) ($value instanceof Slug ? $value : Slug::fromString($value));
    }

    public function getTitleAttribute(string $value): Title
    {
        return Title::fromString($value);
    }

    public function setTitleAttribute(Title|string $value): void
    {
        $this->attributes['title'] = (string) ($value instanceof Title ? $value : Title::fromString($value));
    }

    public function getContentAttribute(string $value): Content
    {
        return Content::fromString($value);
    }

    public function setContentAttribute(Content|string $value): void
    {
        $this->attributes['content'] = (string) ($value instanceof Content ? $value : Content::fromString($value));
    }

    public function toMarkdownFileName(): string
    {
        return $this->slug . '.md';
    }

    public function getMarkdownFilePath(): string
    {
        return 'pages/' . $this->toMarkdownFileName();
    }
}
