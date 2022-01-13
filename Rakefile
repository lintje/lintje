# frozen_string_literal: true

require "erb"
require "json"
require "dotenv"

PROJECT_NAME = "Lintje"
PROJECT_SLUG = "lintje"
PROJECT_MAINTAINER = "Tom de Bruijn tom@tomdebruijn.com"
PROJECT_HOMEPAGE = "https://github.com/tombruijn/lintje"
PROJECT_DESCRIPTION = "Lintje is an opinionated linter for Git."
CLOUDSMITH_REPO = "lintje/lintje"

BUILDS = {
  "x86_64-apple-darwin" => {
    :builder => :cargo,
    :artifact_filename => "lintje",
    :strip => { :local => true }
  },
  "aarch64-apple-darwin" => {
    :builder => :cargo,
    :artifact_filename => "lintje",
    :strip => { :local => true }
  },
  "x86_64-unknown-linux-gnu" => {
    :builder => :cross,
    :artifact_filename => "lintje",
    :platform => "amd64",
    :strip => { :container => true }
  },
  "aarch64-unknown-linux-gnu" => {
    :builder => :cross,
    :artifact_filename => "lintje",
    :platform => "arm64",
    :strip => { :container => true }
  },
  "x86_64-unknown-linux-musl" => {
    :builder => :cross,
    :artifact_filename => "lintje",
    :platform => "amd64",
    :strip => { :container => true }
  },
  "aarch64-unknown-linux-musl" => {
    :builder => :cross,
    :artifact_filename => "lintje",
    :platform => "arm64",
    :strip => { :container => true }
  },
  "x86_64-pc-windows-gnu" => {
    :builder => :cross,
    :artifact_filename => "lintje.exe",
    :platform => "amd64",
    :strip => { :container => "rustembedded/cross:x86_64-pc-windows-gnu" }
  }
}.freeze

DIST_DIR = "dist"
DIST_ARCHIVES_DIR = File.join(DIST_DIR, "archives")
DIST_PACKAGES_DIR = File.join(DIST_DIR, "packages")
ARCHIVES_CHECKSUMS_FILE = File.join(DIST_ARCHIVES_DIR, "checksums_256.txt")

namespace :build do
  task :prepare do
    puts "Installing cross if not installed"
    run "which cross > /dev/null 2>&1 || cargo install cross"
  end

  task :all => :prepare do
    clean_dist_dir
    BUILDS.each do |triple, options|
      build_release triple, options
    end
  end

  def build_release(triple, options)
    filename = options[:artifact_filename]
    puts "Building #{triple} (#{filename})"
    prepare_dist_for triple
    run "rustup target add #{triple}"
    run "#{options[:builder]} build --release --target #{triple}"
    FileUtils.copy(
      File.join("target", triple, "release", filename),
      File.join(DIST_ARCHIVES_DIR, triple)
    )
    strip_artifact triple, options
  end

  def strip_artifact(triple, options)
    filename = options[:artifact_filename]
    if options[:strip][:local]
      run "strip #{filename}", :chdir => File.join(DIST_ARCHIVES_DIR, triple)
    elsif options[:strip][:container]
      platform = options[:platform]
      image =
        case options[:strip][:container]
        when true
          # Build development image
          tag = "tombruijn/lintje-#{triple}:build"
          build_docker_image tag, "Dockerfile.#{triple}", :platform => platform
          tag
        else
          # Use existing image
          options[:strip].fetch(:container)
        end

      run_in_container image, <<~COMMAND, :platform => platform
        strip #{File.join(DIST_ARCHIVES_DIR, triple, filename)}
      COMMAND
    else
      raise "No strip method defined for: #{triple}"
    end
  end
end
task :build => ["build:all"]

namespace :release do
  task :prepare => [
    :check_local_changes,
    :check_env,
    :prompt_confirmation,
    :check_tag_presence,
    :check_gh_install
  ]

  task :check_local_changes do
    if local_changes?
      puts "Local changes detected!"
      puts "Please commit all changes before release."
      exit 1
    end
  end

  task :check_env do
    Dotenv.load
    unless ENV["CLOUDSMITH_API_KEY"]
      puts "The CLOUDSMITH_API_KEY env var is not configured in the `.env` " \
        "file."
      puts "Please make sure the environment is configured correctly."
      exit 1
    end
  end

  task :prompt_confirmation do
    version = fetch_package_version
    cargo_lock = File.read("Cargo.lock")
    cargo_lock_updated = cargo_lock.include?(<<~LOCK)
      name = "lintje"
      version = "#{version}"
    LOCK
    unless cargo_lock_updated
      puts "Cargo.lock is not updated to be the same version as Cargo.toml! " \
        "Run `cargo build` to update the lock file."
      exit 1
    end

    answer = prompt_confirmation \
      "Do you want to publish Lintje v#{version}? (y/n) "
    unless answer
      puts "Exiting..."
      exit 0
    end
  end

  task :check_tag_presence do
    version = fetch_package_version
    if run("git tag").split("\n").include?("v#{version}")
      puts "Tag #{version} already exists. Exiting."
      puts "Please make sure to update the version in the Cargo.toml file."
      puts "Don't forget to update the CHANGELOG.md file."
      exit 1
    end
  end

  task :check_gh_install do
    run "which gh &>/dev/null"
  rescue CommandFailed
    puts "The GitHub CLI could not be found. " \
      "Please install it before continuing."
    puts "https://cli.github.com/manual/"
    puts "And run 'gh auth login'"
    exit 1
  end

  task :all => [:prepare, "build:all"] do
    build_archives
    build_packages

    version = fetch_package_version
    tag = "v#{version}"
    run "git tag #{tag}"
    run "git push origin #{current_branch} #{tag}"
    puts "Creating release on GitHub. Please follow the prompts."
    system <<~COMMAND
      gh release create v#{version} \
        #{File.join(DIST_ARCHIVES_DIR, "*.tar.gz")} #{ARCHIVES_CHECKSUMS_FILE} \
        --title "Release #{version}"
    COMMAND

    puts "Publishing to crates.io"
    system "cargo publish"

    puts "Release of version #{version} done!"
    puts "Please update the Homebrew tap next: https://github.com/tombruijn/homebrew-lintje"
  end

  task :archives do
    build_archives
  end

  def build_archives
    puts "Building release archives"
    prepare_checksums_file
    BUILDS.each do |triple, _options|
      archive_artifact triple
      add_archive_checksum triple
    end
  end

  def archive_artifact(triple)
    archive_location = File.join("..", "#{triple}.tar.gz")
    run "tar -cvzf #{archive_location} *",
      :chdir => File.join(DIST_ARCHIVES_DIR, triple)
  end

  def prepare_checksums_file
    if File.exist? ARCHIVES_CHECKSUMS_FILE
      FileUtils.remove ARCHIVES_CHECKSUMS_FILE
    end
    FileUtils.touch ARCHIVES_CHECKSUMS_FILE
  end

  def add_archive_checksum(triple)
    checksum = run(
      "shasum -a 256 #{triple}.tar.gz",
      :chdir => DIST_ARCHIVES_DIR
    )
    # Append checksum
    File.open(ARCHIVES_CHECKSUMS_FILE, "a") do |file|
      file.write checksum
    end
  end

  task :packages do
    build_packages
  end

  def build_packages
    puts "Building OS packages"
    build_debian_package "amd64", "x86_64-unknown-linux-gnu"
    build_debian_package "arm64", "aarch64-unknown-linux-gnu"
  end

  def build_debian_package(package_architecture, triple)
    package_config = BUILDS[triple]
    package_revision = "1" # Hardcoded package revision for now
    version = fetch_package_version
    package_name =
      "#{PROJECT_SLUG}-#{version}-#{package_revision}-#{package_architecture}"
    package_filename = "#{package_name}.deb"

    # Prepare packge dist dir
    package_dir = File.join(DIST_PACKAGES_DIR, package_name)
    dist_dir = File.join(DIST_ARCHIVES_DIR, triple)
    FileUtils.remove_dir package_dir if Dir.exist? package_dir
    FileUtils.mkdir_p package_dir

    # Create DEBIAN `control` file with package metadata
    debian_dir = File.join(package_dir, "DEBIAN")
    FileUtils.mkdir_p debian_dir
    File.open File.join(debian_dir, "control"), "w" do |file|
      bind = Class.new do
        def initialize(version, architecture)
          @package_slug = PROJECT_NAME
          @package_name = PROJECT_NAME
          @package_version = version
          @package_architecture = architecture
          @package_maintainer = PROJECT_MAINTAINER
          @package_description = PROJECT_DESCRIPTION
        end

        def fetch_binding
          binding
        end
      end.new(version, package_architecture).fetch_binding
      template = File.read("support/packages/deb/DEBIAN/control.erb")
      file.write ERB.new(template).result bind
    end
    # Copy executable to package dist dir.
    # The path inside the package dist dir mimics the install location on the
    # installation machine.
    bin_dir = File.join(package_dir, "usr", "bin")
    FileUtils.mkdir_p bin_dir
    FileUtils.cp(
      File.join(dist_dir, package_config[:artifact_filename]),
      bin_dir
    )

    # Build the Docker image in which to build the package
    image_tag = "ubuntu-deb:build_#{package_architecture}"
    build_docker_image image_tag,
      "Dockerfile.ubuntu-deb",
      :platform => package_architecture
    # Build and test the package
    run_in_container image_tag,
      "support/script/build_deb",
      :platform => package_architecture,
      :env => { "PACKAGE_NAME" => package_name }
    upload_debian_package(package_filename)
  end

  def upload_debian_package(filename)
    api_key = ENV["CLOUDSMITH_API_KEY"]
    file_path = File.join(DIST_PACKAGES_DIR, filename)
    # Upload the package to Cloudsmith
    response = run <<~COMMAND
      curl \
        --silent \
        --show-error \
        --upload-file #{file_path} \
        -u 'tombruijn:#{api_key}' \
        -H "Content-Sha256: $(shasum -a256 '#{file_path}' | cut -f1 -d' ')" \
        https://upload.cloudsmith.io/#{CLOUDSMITH_REPO}/#{filename}
    COMMAND
    output = JSON.parse(response)
    identifier = output.fetch("identifier")
    # Create package on Cloudsmith
    # It's currently set to any-distro/any-version, because the executable has
    # no specific dependencies or limitations for older versions of
    # distributions that I know of.
    run <<~COMMAND
      curl -X POST -H "Content-Type: application/json" \
        -u 'tombruijn:#{api_key}' \
        -d '{"package_file": "#{identifier}", "distribution": "any-distro/any-version"}' \
        https://api-prd.cloudsmith.io/v1/packages/#{CLOUDSMITH_REPO}/upload/deb/
    COMMAND
  end
end
task :release => ["release:all"]

def clean_dist_dir
  FileUtils.remove_dir DIST_DIR
end

def prepare_dist_for(triple)
  FileUtils.mkdir_p(DIST_ARCHIVES_DIR)
  target_dist_dir = File.join(DIST_ARCHIVES_DIR, triple)
  FileUtils.remove_dir(target_dist_dir) if Dir.exist? target_dist_dir
  FileUtils.mkdir_p(target_dist_dir)
end

# Run a command
#
# Outputs the STDOUT and STDERR while it's running and returns the STDOUT AND
# STDERR as the method return value.
def run(command, chdir: nil)
  chdir_label = " (#{chdir})" if chdir
  puts "Running command: #{command}#{chdir_label}"
  read, write = IO.pipe
  options = { [:out, :err] => write }
  options[:chdir] = chdir if chdir
  pid = spawn command, options
  output_lines = []
  thread =
    Thread.new do
      while line = read.readline # rubocop:disable Lint/AssignmentInCondition
        # Output lines as the program runs
        puts "| #{line}"
        # Store the output for later
        output_lines << line
      end
    rescue EOFError
      # Do nothing, nothing to read anymore
    end
  _pid, status = Process.wait2 pid
  write.close
  thread.join
  output = output_lines.join
  raise CommandFailed.new(command, output) unless status.success?

  puts
  output
end

def build_docker_image(image, dockerfile, platform: nil)
  puts "Building docker image: #{image} (support/docker/#{dockerfile})"
  platform_option = "--platform=#{platform_for_docker platform}" if platform
  run <<~COMMAND, :chdir => "support/docker"
    docker build \
      #{platform_option} \
      --file #{dockerfile} \
      --tag #{image} \
      .
  COMMAND
end

def run_in_container(image, command, env: nil, platform: nil)
  platform_option = "--platform=#{platform_for_docker platform}" if platform
  if env
    mapped_env = env.map { |key, value| "#{key}=#{value}" }.join(" ")
    env_option = "--env #{mapped_env}"
  end
  run <<~COMMAND
    docker run \
      --rm \
      -it \
      --volume "#{__dir__}:/project" \
      --workdir "/project" \
      #{platform_option} \
      #{env_option} \
      #{image} \
      #{command}
  COMMAND
end

def platform_for_docker(platform)
  case platform
  when "amd64"
    "linux/amd64"
  when "arm64"
    "linux/arm64/v8"
  else
    raise "Unknown platform: #{platform}"
  end
end

def fetch_package_version
  File.read("Cargo.toml").scan(/version = "(.*)"/).first.first
end

def local_changes?
  `git status -s -u`.split("\n").each do |change|
    change.gsub!(/^.. /, "")
  end.any?
end

def current_branch
  `git rev-parse --abbrev-ref HEAD`.chomp
end

def prompt_confirmation(message)
  loop do
    print message
    input = fetch_input.strip
    case input
    when "y", "Y", "yes"
      return true
    when "n", "N", "no"
      return false
    end
  end
end

def fetch_input
  input = $stdin.gets
  input ? input.chomp : ""
rescue Interrupt
  puts "\nExiting..."
  exit 1
end

class CommandFailed < StandardError
  def initialize(command, output)
    @command = command
    @output = output
    super()
  end

  def message
    "The command has failed to run: #{@command}\nOutput:\n#{@output}"
  end
end
