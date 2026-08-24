<?php

declare(strict_types=1);

namespace App\Features\Wiki\ValueObjects;

use App\Features\Wiki\Exceptions\InvalidSlugException;

final class Slug
{
    private string $value;

    private function __construct(string $value)
    {
        $this->value = $value;
    }

    public static function fromString(string $value): self
    {
        $value = trim($value);
        if ($value === '') {
            throw new InvalidSlugException('Slug cannot be empty.');
        }
        if (strlen($value) > 255) {
            throw new InvalidSlugException('Slug cannot exceed 255 characters.');
        }
        if (!preg_match('/^[a-z0-9\-_]+$/', $value)) {
            throw new InvalidSlugException(
                'Slug must contain only lowercase letters, digits, hyphens, and underscores.'
            );
        }
        return new self($value);
    }

    public function toString(): string
    {
        return $this->value;
    }

    public function __toString(): string
    {
        return $this->value;
    }
}
