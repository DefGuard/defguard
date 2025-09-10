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
      name                = "debian-13-amd64-*"
      root-device-type    = "ebs"
      virtualization-type = "hvm"
    }
    most_recent = true
    owners      = ["136693071363"]
  }
  ssh_username = "admin"
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
    script = "./images/ami/core.sh"
  }

  provisioner "shell" {
    inline = ["rm /home/admin/.ssh/authorized_keys"]
  }

  provisioner "shell" {
    inline = ["sudo rm /root/.ssh/authorized_keys"]
  }
}
