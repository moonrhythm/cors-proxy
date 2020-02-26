build:
	docker build -t gcr.io/moonrhythm-containers/cors-proxy .

publish: build
	docker push gcr.io/moonrhythm-containers/cors-proxy
