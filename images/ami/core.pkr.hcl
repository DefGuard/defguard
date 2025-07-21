packer {
  required_plugins {
    amazon = {
      version = ">= 1.2.8"
      source  = "github.com/hashicorp/amazon"
    }
  }
}

variable "package_version" {
  type = string
}

variable "region" {
  type    = string
  default = "eu-north-1"
}

variable "instance_type" {
  type    = string
  default = "t3.micro"
}

source "amazon-ebs" "defguard-core" {
  ami_name      = "defguard-core-${var.package_version}-amd64"
  instance_type = var.instance_type
  region        = var.region
  source_ami_filter {
    filters = {
      name                = "ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*"
      root-device-type    = "ebs"
      virtualization-type = "hvm"
    }
    most_recent = true
    owners      = ["099720109477"]
  }
  ssh_username = "ubuntu"
}

build {
  name = "defguard-core"
  sources = [
    "source.amazon-ebs.defguard-core"
  ]
  
  provisioner "file" {
    source      = "defguard-${var.package_version}-x86_64-unknown-linux-gnu.deb"
    destination = "/tmp/defguard-core.deb"
  }

  provisioner "shell" {
    script = "core.sh"
  }

  provisioner "shell" {
    inline = ["rm /home/ubuntu/.ssh/authorized_keys"]
  }

  provisioner "shell" {
    inline = ["sudo rm /root/.ssh/authorized_keys"]
  }
}
