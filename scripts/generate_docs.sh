#!/bin/bash
set -e

# Colors for terminal output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

DOCS_DIR="target/doc"
PUBLIC_DIR="docs"

# Parse command line arguments
OPEN_DOCS=false
PREPARE_PUBLISH=false

for arg in "$@"; do
  case $arg in
    --open)
      OPEN_DOCS=true
      shift
      ;;
    --publish)
      PREPARE_PUBLISH=true
      shift
      ;;
  esac
done

echo -e "${BLUE}Generating Rust documentation...${NC}"

# Generate documentation
cargo doc --no-deps

# Add a redirect index.html file at the top level
echo -e "${GREEN}Creating index.html redirect...${NC}"
echo '<meta http-equiv="refresh" content="0; url=phala_tee_deploy_rs/index.html">' > $DOCS_DIR/index.html

if [ "$PREPARE_PUBLISH" = true ]; then
  echo -e "${YELLOW}Preparing documentation for publishing...${NC}"
  
  # Create public directory if it doesn't exist
  mkdir -p $PUBLIC_DIR
  
  # Copy documentation to public directory
  cp -R $DOCS_DIR/* $PUBLIC_DIR/
  
  echo -e "${GREEN}Documentation prepared for publishing in the '$PUBLIC_DIR' directory${NC}"
fi

if [ "$OPEN_DOCS" = true ]; then
  echo -e "${GREEN}Opening documentation in your browser...${NC}"
  
  # Open the documentation in the default browser (cross-platform)
  case "$(uname -s)" in
    Darwin*)  open $DOCS_DIR/index.html ;; # macOS
    Linux*)   xdg-open $DOCS_DIR/index.html ;; # Linux
    CYGWIN*|MINGW*|MSYS*)  start $DOCS_DIR/index.html ;; # Windows
    *)        echo "Unknown operating system. Please open $DOCS_DIR/index.html manually." ;;
  esac
fi

echo -e "${GREEN}Documentation generated successfully!${NC}"
echo -e "Location: ${BLUE}$DOCS_DIR/phala_tee_deploy_rs/index.html${NC}" 