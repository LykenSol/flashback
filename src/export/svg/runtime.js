(function() {
    function int(x) {
        return x | 0;
    }

    function svg_element(tag) {
        return document.createElementNS('http://www.w3.org/2000/svg', tag);
    }

    function Timeline(data, container) {
        if(!(this instanceof Timeline))
            return new Timeline(data);

        this.frame_count = data.frame_count;
        this.named = Object.create(null);
        this.actions = data.actions;
        this.layers = data.layers.map(function(frames, i) {
            var container = svg_element('g');
            var use = svg_element('use');
            container.appendChild(use);
            return { frames: frames, container: container, use: use };
        });
        this.container = container;
        this.attachLayers();
    }
    Timeline.prototype.paused = false;
    Timeline.prototype.frame = 0;
    Timeline.prototype.attachLayers = function() {
        var container = this.container;
        this.layers.forEach(function(layer) {
            container.appendChild(layer.container);
        });
    };
    Timeline.prototype.detachLayers = function() {
        this.layers.forEach(function(layer) {
            layer.container.remove();
        });
    };
    Timeline.prototype.showFrame = function() {
        if(this.paused)
            return;

        var frame = this.frame;
        var named = this.named;
        this.layers.forEach(function(layer, depth) {
            var obj = layer.frames[frame];

            // TODO(eddyb) this might need to take SWF's `is_move` into account.
            // Remove the old character if necessary.
            if(obj === null || (obj && layer.character != obj.character)) {
                layer.character = -1;
                layer.use.removeAttribute('href');
                if(layer.sprite) {
                    layer.sprite.detachLayers();
                    layer.sprite = null;
                }
            }

            // Remove the old name if necessary.
            if(obj === null || (obj && layer.name != obj.name)) {
                named[layer.name] = null;
                layer.name = null;
            }

            if(obj) {
                if(layer.character != obj.character) {
                    layer.character = obj.character;
                    layer.use.setAttribute('href', '#c_' + obj.character);

                    var sprite_data = sprites[obj.character];
                    if(sprite_data)
                        layer.sprite = new Timeline(sprite_data, layer.container);
                }
                layer.container.setAttribute('transform', 'matrix(' + obj.matrix.join(' ') + ')');
                if(layer.name != obj.name) {
                    layer.name = obj.name;
                    named[layer.name] = depth;
                }
            }

            // Update the sprite if it exists.
            if(layer.sprite)
                layer.sprite.showFrame();
        });

        var action = this.actions[frame];
        if(action)
            action(new ActionRuntime(this));

        this.frame = (frame + 1) % this.frame_count;
    };

    function ActionRuntime(timeline) {
        if(!(this instanceof ActionRuntime))
            return new ActionRuntime(data);

        this.timeline = timeline;
    }
    ActionRuntime.prototype.play = function() {
        this.timeline.paused = false;
    };
    ActionRuntime.prototype.stop = function() {
        this.timeline.paused = true;
    };
    ActionRuntime.prototype.gotoFrame = function(frame) {
        this.timeline.frame = frame;
    };
    ActionRuntime.prototype.getVar = function(name) {
        var depth = this.timeline.named[name];
        if(depth) {
            var layer = this.timeline.layers[depth];
            var api = Object.create(null);
            if(layer.sprite) {
                api.gotoAndPlay = function(frame) {
                    layer.sprite.frame = frame;
                    layer.sprite.paused = false;
                };
            }
            return api;
        }

        console.error('trying to get var', name);
    };
    ActionRuntime.prototype.setVar = function(name, value) {
        console.error('trying to set var', name, 'to', value);
    };
    ActionRuntime.prototype.getFn = function(name) {
        if(name === 'hasOwnProperty')
            return function(o, x) {
                return Object.prototype.hasOwnProperty.call(o, x);
            };
        console.error('trying to get fn', name);
    };

    timeline = new Timeline(timeline, document.getElementById('body'));

    var start;
    var last_frame = 0;
    function update(now) {
        window.requestAnimationFrame(update);

        if(!start) start = now;
        // TODO(eddyb) figure out how to avoid absolute values.
        var frame = int((now - start) * frame_rate / 1000);

        for(; last_frame < frame; last_frame++)
            timeline.showFrame();
    }
    window.requestAnimationFrame(update);
})()
