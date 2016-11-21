.PHONY: image

IMAGE_NAME ?= codeclimate/codeclimate-rustfmt

image:
	docker build --rm -t $(IMAGE_NAME) .
