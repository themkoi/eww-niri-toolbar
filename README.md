# eww-niri-toolbar

A rust binary that outputs app information from niri-ipc to be consumed by eww.

## Example widget
```clojure
(deflisten wlr_apps :initial "[]" "$PATH_TO_BINARY/eww-niri-taskbar")

(defwidget taskbar []
  (box
    :orientation "h"
    :space-evenly false
    :spacing 10
    :class "taskbar"
    (for ws in {wlr_apps.workspaces}
      (box
        :orientation "h"
        :class "workspace"
        (for app in {ws.windows}
          (button
            :class "app_item ${app.is_focused == true ? "active" : ""}"
            :onclick "niri msg action focus-window --id ${app.id}"
            :onmiddleclick "niri msg action close-window --id ${app.id}"
            (box :orientation "h" :class "app_image"
              (image :path "${app.icon_path}" :image-width 16))
          )
        )
      )
    )
  )
)
```
