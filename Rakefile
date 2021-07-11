# frozen_string_literal: true

require "erb"

PROJECT_NAME = "Lintje"
PROJECT_SLUG = "lintje"
PROJECT_MAINTAINER = "Tom de Bruijn tom@tomdebruijn.com"
PROJECT_HOMEPAGE = "https://github.com/tombruijn/lintje"
PROJECT_DESCRIPTION = "Lintje is an opinionated linter for Git."

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
  task :check_local_changes do
    if local_changes?
      puts "Local changes detected!"
      puts "Please commit all changes before release."
      exit 1
    end
  end

  task :prepare do
    run "which gh &>/dev/null"
  rescue CommandFailed
    puts "The GitHub CLI could not be found. " \
      "Please install it before continuing."
    puts "https://cli.github.com/manual/"
    puts "And run 'gh auth login'"
    exit 1
  end

  task :all => [:check_local_changes, :prepare, "build:all"] do
    version = fetch_package_version
    if run("git tag").split("\n").include?("v#{version}")
      puts "Tag #{version} already exists. Exiting."
      puts "Please make sure to update the version in the Cargo.toml file."
      puts "Don't forget to update the CHANGELOG.md file."
      exit 1
    end

    answer = prompt_confirmation \
      "Do you want to publish Lintje v#{version}? (y/n) "
    unless answer
      puts "Exiting..."
      exit 0
    end

    build_archives

    tag = "v#{version}"
    run "git tag #{tag}"
    run "git push origin #{current_branch} #{tag}"
    puts "Creating release on GitHub. Please follow the prompts."
    system <<~COMMAND
      gh release create v#{version} \
        #{File.join(DIST_ARCHIVES_DIR, "*.tar.gz")} #{ARCHIVES_CHECKSUMS_FILE} \
        --title "Release #{version}"
    COMMAND
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
        # Output lines as the program runes
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
