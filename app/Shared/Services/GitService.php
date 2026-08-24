<?php

declare(strict_types=1);

namespace App\Shared\Services;

use Illuminate\Support\Facades\Storage;
use Symfony\Component\Process\Process;
use RuntimeException;

class GitService
{
    protected string $repoPath;

    public function __construct()
    {
        $this->repoPath = storage_path('app/wiki-repo');
        $this->ensureRepoExists();
    }

    protected function ensureRepoExists(): void
    {
        if (!is_dir($this->repoPath)) {
            mkdir($this->repoPath, 0755, true);
            $this->initRepo();
        }
    }

    protected function initRepo(): void
    {
        $process = new Process(['git', 'init', $this->repoPath]);
        $process->run();
        if (!$process->isSuccessful()) {
            throw new RuntimeException('Failed to initialize Git repository.');
        }
        $this->runGitCommand(['config', 'user.email', 'tuxpedia@example.com']);
        $this->runGitCommand(['config', 'user.name', 'Tuxpedia']);
    }

    protected function runGitCommand(array $args): string
    {
        $cmd = array_merge(['git', '-C', $this->repoPath], $args);
        $process = new Process($cmd);
        $process->run();
        if (!$process->isSuccessful()) {
            throw new RuntimeException(
                'Git command failed: ' . $process->getErrorOutput()
            );
        }
        return $process->getOutput();
    }

    public function commitFile(string $filePath, string $content, string $commitMessage): void
    {
        $fullPath = $this->repoPath . '/' . $filePath;
        $dir = dirname($fullPath);
        if (!is_dir($dir)) {
            mkdir($dir, 0755, true);
        }
        file_put_contents($fullPath, $content);

        $this->runGitCommand(['add', $filePath]);
        $this->runGitCommand(['commit', '-m', $commitMessage]);
    }

    public function moveFile(string $from, string $to): void
    {
        $this->runGitCommand(['mv', $from, $to]);
        $this->runGitCommand(['commit', '-m', "Move file from {$from} to {$to}"]);
    }

    public function deleteFile(string $filePath, string $commitMessage): void
    {
        $this->runGitCommand(['rm', $filePath]);
        $this->runGitCommand(['commit', '-m', $commitMessage]);
    }

    public function getHistory(string $filePath): array
    {
        $output = $this->runGitCommand([
            'log', '--oneline', '--follow', '--', $filePath
        ]);
        return array_filter(explode("\n", trim($output)));
    }
}
