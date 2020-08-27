// vue.config.js
module.exports = {
    // options...
    devServer: {
        disableHostCheck: true,
	proxy: {
		"/api/*": {
			target: "http://[::1]:8000",
			secure: false,
			changeOrigin: true
	        },
		"/img/*": {
			target: "http://[::1]:8000",
			secure: false,
			changeOrigin: true
		}
        }
    }
}
