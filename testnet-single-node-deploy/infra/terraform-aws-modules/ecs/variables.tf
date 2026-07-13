variable "environment" {
  default = "dev"
}

variable "region" {
  default = "eu-central-1"
}

variable "account_id" {
}

variable "name" {
}

variable "vpc_id" {
}

variable "private_subnets" {
  type = list(string)
}

variable "public_subnets" {
  type = list(string)
}

variable "image" {
}

variable "task_cpu" {
  default = 4096
  type    = number
}

variable "task_memory" {
  default = 16384
  type    = number
}

variable "container_health_check_command" {
  description = "Container health check command to keep task definition changes explicit"
  type        = list(string)
  default = [
    "CMD-SHELL",
    "curl -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"getinfo\",\"params\":[],\"id\":1}' http://localhost:18232/ || exit 1"
  ]
}

variable "enable_logging" {
  default = false
  type    = bool
}

variable "enable_domain" {
  description = "Flag to enable domain specific configurations"
  default     = false
  type        = bool
}

variable "domain" {
  default = "false.com"
}

variable "zone_name" {
  default = ""
  type    = string
}

variable "enable_persistent" {
  description = "A flag to enable or disable the creation of a persistent volume"
  default     = true
  type        = bool
}

variable "enable_backup" {
  description = "A flag to enable or disable the creation of a backup policy"
  default     = false
  type        = bool
}

variable "port_mappings" {
  type = list(object({
    containerPort = number
    hostPort      = number
    protocol      = string
  }))
  description = "List of port mappings"
}

variable "persistent_volume_size" {
  description = "The size of the persistent volume"
  default     = 40
  type        = number
}

variable "efs_encrypted" {
  description = "Whether to encrypt the persistent EFS file system"
  default     = true
  type        = bool
}

variable "allow_all_ecs_ingress" {
  description = "Allow all ingress to the ECS service security group"
  default     = false
  type        = bool
}

variable "allow_all_efs_ingress" {
  description = "Allow all NFS ingress to the EFS security group"
  default     = false
  type        = bool
}

variable "create_legacy_lb_security_group" {
  description = "Keep the legacy load balancer security group managed for existing deployments"
  default     = false
  type        = bool
}
