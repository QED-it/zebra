locals {
  # Automatically load environment-level variables
  environment_vars = read_terragrunt_config(find_in_parent_folders("env.hcl"))
  account_vars     = read_terragrunt_config(find_in_parent_folders("account.hcl"))

  # Extract out common variables for reuse
  env = local.environment_vars.locals.environment
}

# Terragrunt will copy the Terraform configurations specified by the source parameter, along with any files in the
# working directory, into a temporary folder, and execute your Terraform commands in that folder.
terraform {
  source = "git::https://github.com/terraform-aws-modules/terraform-aws-vpc.git//?ref=v6.6.1"
}

# Include all settings from the root terragrunt.hcl file
include {
  path = find_in_parent_folders()
}

inputs = {
  name = "${local.env}-tf-vpc"
  cidr = local.environment_vars.locals.cidr

  azs             = local.environment_vars.locals.availability_zones
  private_subnets = local.environment_vars.locals.internal_subnets
  public_subnets  = local.environment_vars.locals.external_subnets

  create_database_subnet_group = false

  enable_dns_hostnames   = true
  enable_dns_support     = true
  enable_nat_gateway     = true
  single_nat_gateway     = false
  one_nat_gateway_per_az = false

  manage_default_security_group  = true
  default_security_group_ingress = []
  default_security_group_egress = [
    {
      cidr_blocks = "0.0.0.0/0"
    }
  ]

  tags = {
    Environment = local.env
    Name        = "${local.env}-tf-vpc"
  }
}
