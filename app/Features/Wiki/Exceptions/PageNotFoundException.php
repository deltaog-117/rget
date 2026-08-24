<?php

declare(strict_types=1);

namespace App\Features\Wiki\Exceptions;

use RuntimeException;

class PageNotFoundException extends RuntimeException
{
    public function __construct(string $slug)
    {
        parent::__construct("Page with slug '{$slug}' not found.");
    }
}
