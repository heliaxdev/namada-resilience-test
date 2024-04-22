tag = v0.31.9

build-config:
	docker build --no-cache -t namada-config:${tag} -f config/Dockerfile config

build-genesis:
	docker build --no-cache -t namada-genesis:${tag} -f genesis/Dockerfile genesis

build-namada:
	docker build --no-cache -t namada:${tag} -f namada/Dockerfile namada

build-namada-inst:
	docker build --no-cache -t namada:${tag}-inst -f namada/Dockerfile.inst namada