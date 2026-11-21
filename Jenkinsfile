// Multibranch Pipeline: BRANCH_NAME is required, never guessed from the worktree.
pipeline {
    agent { label 'rust_no_docker' }

    options {
        skipDefaultCheckout(true)
        disableConcurrentBuilds()
        timeout(time: 45, unit: 'MINUTES')
    }

    environment {
        PYTHONUNBUFFERED = '1'
        PYTHONDONTWRITEBYTECODE = '1'
        NO_COLOR = '1'
        DECKSTRIDE_HEADLESS_CI = '1'
        CARGO_TERM_COLOR = 'never'
        BADGE_ROOT = '/mnt/assets/coverage'
    }

    stages {
        stage('Checkout') {
            steps {
                deleteDir()
                checkout scm
                sh 'uv run --no-project --python ">=3.11" python scripts/ci.py init'
            }
        }

        stage('Tools') {
            steps {
                sh '''
                    set -eu
                    uv run --no-project --python ">=3.11" python -c 'import sys; assert sys.version_info >= (3, 11), "Python 3.11+ required for CI"'
                    test -n "${BRANCH_NAME:?Configure this job as a Multibranch Pipeline}"
                    uname -sr
                    rustc --version
                    if ! cargo llvm-cov --version | grep -Fx 'cargo-llvm-cov 0.9.1'; then
                        cargo install cargo-llvm-cov --version 0.9.1 --locked
                    fi
                    rustup component add llvm-tools-preview clippy rustfmt
                '''
            }
        }

        stage('Format, lint and docs') {
            steps {
                sh '''
                    set -eu
                    cargo fmt --all --check
                    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
                    RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps
                '''
            }
        }

        stage('Tests') {
            steps {
                // Keep failures visible while allowing independent coverage measurement.
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE', catchInterruptions: false) {
                    sh '''
                        set -eu
                        uv run --no-project --python ">=3.11" python -m unittest discover -s tests -v
                        unset DECKSTRIDE_GUI_TESTS
                        cargo test --locked --workspace --all-features
                        cargo build --locked --workspace --all-features
                        uv run --no-project --python ">=3.11" python scripts/ci.py tests-passed
                    '''
                }
            }
        }
        stage('Coverage') {
            steps {
                sh '''
                    set -eu
                    unset DECKSTRIDE_GUI_TESTS
                    uv run --no-project --python ">=3.11" python scripts/ci.py coverage
                '''
            }
        }
    }

    post {
        cleanup {
            script {
                sh 'uv run --no-project --python ">=3.11" python scripts/ci.py finalize'
                archiveArtifacts artifacts: 'coverage/version.svg,coverage/test.svg,coverage/coverage.svg',
                    fingerprint: true, allowEmptyArchive: false
                if (env.BRANCH_NAME == 'main' && !env.CHANGE_ID) {
                    sh 'scripts/publish-badges.sh "$BADGE_ROOT" version test coverage'
                }
                echo "Tests, coverage diagnostics and badges finalized; result: ${currentBuild.currentResult}. No release created."
            }
        }
    }
}
