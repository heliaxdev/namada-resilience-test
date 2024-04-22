build-config:
	docker build --no-cache -t namada-config:${sha} -f --build-arg GIT_SHA=${sha} config/Dockerfile config

build-genesis:
	docker build --no-cache -t namada-genesis:${sha} -f genesis/Dockerfile --build-arg GIT_SHA=${sha} genesis

build-namada:
	docker build --no-cache -t namada:${sha} -f namada/Dockerfile --build-arg GIT_SHA=${sha} namada

build-namada-inst:
	docker build --no-cache -t namada:${sha}-inst -f namada/Dockerfile.inst --build-arg GIT_SHA=${sha} namada