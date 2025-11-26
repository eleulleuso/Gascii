#!/bin/zsh
# Build script for Bad Apple ASCII Animation
# Author: BadApple C Team
# Date: 2025-07-05

set -e  # Exit on any error

# 색상 정의
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 로깅 함수들
log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${PURPLE}[STEP]${NC} $1"
}

# 헤더 출력
print_header() {
    echo -e "${BLUE}"
    echo "┌─────────────────────────────────────────────────────────────┐"
    echo "│                 🍎 Bad Apple Build Script                   │"
    echo "│              High Performance C Implementation              │"
    echo "│                     Version 1.0                            │"
    echo "└─────────────────────────────────────────────────────────────┘"
    echo -e "${NC}"
}

# 시스템 정보 확인
check_system() {
    log_step "Checking system information..."
    
    echo "System: $(uname -s) $(uname -m)"
    echo "Shell: $SHELL"
    echo "Compiler: $(gcc --version | head -n1)"
    echo "Make: $(make --version | head -n1)"
    echo "CPU cores: $(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo "unknown")"
    
    log_success "System check completed"
}

# 의존성 확인
check_dependencies() {
    log_step "Checking build dependencies..."
    
    local missing_deps=()
    
    # 필수 의존성 확인
    command -v gcc >/dev/null 2>&1 || missing_deps+=("gcc")
    command -v make >/dev/null 2>&1 || missing_deps+=("make")
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing required dependencies: ${missing_deps[*]}"
        log_info "Please install missing dependencies and try again"
        exit 1
    fi
    
    # 선택적 의존성 확인
    local optional_deps=()
    command -v ffplay >/dev/null 2>&1 || optional_deps+=("ffplay")
    command -v valgrind >/dev/null 2>&1 || optional_deps+=("valgrind")
    command -v cppcheck >/dev/null 2>&1 || optional_deps+=("cppcheck")
    
    if [ ${#optional_deps[@]} -ne 0 ]; then
        log_warning "Missing optional dependencies: ${optional_deps[*]}"
        log_info "Some features may not be available"
    fi
    
    log_success "Dependency check completed"
}

# 디렉토리 구조 생성
setup_directories() {
    log_step "Setting up directory structure..."
    
    mkdir -p assets/ascii_frames
    mkdir -p assets/audio
    mkdir -p build/obj/core
    mkdir -p build/obj/utils
    mkdir -p build/bin
    mkdir -p tests
    mkdir -p docs
    
    log_success "Directory structure created"
}

# 샘플 프레임 생성 (테스트용)
generate_sample_frames() {
    log_step "Generating sample ASCII frames for testing..."
    
    local frames_dir="assets/ascii_frames"
    
    if [ ! "$(ls -A $frames_dir 2>/dev/null)" ]; then
        log_info "Creating sample frames for testing..."
        
        for i in {1..10}; do
            local frame_file="$frames_dir/frame_$(printf '%04d' $i).txt"
            cat > "$frame_file" << EOF
╔══════════════════════════════════════════════════════════════════╗
║                        Bad Apple Frame $i                        ║
║                                                                  ║
║    ⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿    ║
║    ⣿⣿⣿⠿⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠛⠿⣿⣿⣿    ║
║    ⣿⣿⠏⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠹⣿⣿    ║
║    ⣿⡏⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢹⣿    ║
║    ⣿⠃⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿    ║
║    ⡏⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢹    ║
║    ⠃⠀⠀⠀⠀⠀⠀⠀⠀⣀⣀⣀⣀⣀⣀⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠸    ║
║    ⠀⠀⠀⠀⠀⠀⠀⠀⣼⣿⣿⣿⣿⣿⣿⣿⣧⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀    ║
║    ⠀⠀⠀⠀⠀⠀⠀⠀⣿⣿⣿⣿⣿⣿⣿⣿⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀    ║
║    ⠀⠀⠀⠀⠀⠀⠀⠀⠻⣿⣿⣿⣿⣿⣿⣿⠟⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀    ║
║    ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠉⠉⠉⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀    ║
║                                                                  ║
║               This is a sample frame for testing                 ║
║                   Replace with actual Bad Apple frames           ║
╚══════════════════════════════════════════════════════════════════╝
EOF
        done
        
        log_success "Sample frames generated (replace with actual Bad Apple frames)"
    else
        log_info "ASCII frames already exist"
    fi
}

# 빌드 실행
build_project() {
    local build_type="$1"
    
    log_step "Building project ($build_type mode)..."
    
    # 성능 측정
    local start_time=$(date +%s.%N)
    
    case "$build_type" in
        "debug")
            make debug -j$(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo "4")
            ;;
        "release"|*)
            make all -j$(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo "4")
            ;;
    esac
    
    local end_time=$(date +%s.%N)
    local build_time=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "unknown")
    
    log_success "Build completed in ${build_time} seconds"
}

# 테스트 실행
run_tests() {
    log_step "Running tests..."
    
    if make test; then
        log_success "All tests passed"
    else
        log_warning "Some tests failed"
    fi
}

# 사용법 출력
print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --debug     Build in debug mode"
    echo "  --release   Build in release mode (default)"
    echo "  --test      Run tests after building"
    echo "  --clean     Clean before building"
    echo "  --install   Install after building"
    echo "  --help      Show this help"
    echo ""
    echo "Examples:"
    echo "  $0                      # Release build"
    echo "  $0 --debug --test       # Debug build with tests"
    echo "  $0 --clean --release    # Clean release build"
}

# 메인 함수
main() {
    local build_type="release"
    local run_test=false
    local clean_first=false
    local install_after=false
    
    # 인수 파싱
    while [[ $# -gt 0 ]]; do
        case $1 in
            --debug)
                build_type="debug"
                shift
                ;;
            --release)
                build_type="release"
                shift
                ;;
            --test)
                run_test=true
                shift
                ;;
            --clean)
                clean_first=true
                shift
                ;;
            --install)
                install_after=true
                shift
                ;;
            --help)
                print_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                print_usage
                exit 1
                ;;
        esac
    done
    
    # 빌드 과정 실행
    print_header
    check_system
    check_dependencies
    setup_directories
    generate_sample_frames
    
    if [ "$clean_first" = true ]; then
        log_step "Cleaning previous build..."
        make clean
        log_success "Clean completed"
    fi
    
    build_project "$build_type"
    
    if [ "$run_test" = true ]; then
        run_tests
    fi
    
    if [ "$install_after" = true ]; then
        log_step "Installing..."
        make install
        log_success "Installation completed"
    fi
    
    # 빌드 완료 메시지
    echo -e "${GREEN}"
    echo "┌─────────────────────────────────────────────────────────────┐"
    echo "│                  🎉 Build Successful! 🎉                   │"
    echo "│                                                             │"
    echo "│  To run: ./build/bin/badapple --help                       │"
    echo "│  Or:     make test                                          │"
    echo "└─────────────────────────────────────────────────────────────┘"
    echo -e "${NC}"
}

# 스크립트 실행
main "$@"
