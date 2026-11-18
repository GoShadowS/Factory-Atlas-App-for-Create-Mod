Пользовательские перерисованные текстуры (User Texture Overrides)
================================================================

Поместите сюда ваши собственные PNG-текстуры с именем по стабильному ID ресурса:
- Замените двоеточие `:` на два подчёркивания `__` (или используйте подпапку по namespace).
Примеры:
  - assets-overrides/create__mechanical_press.png
  - assets-overrides/create/mechanical_press.png
  - assets-overrides/create__andesite_alloy.png
  - assets-overrides/minecraft__iron_ingot.png
  - assets-overrides/createdieselgenerators__distillation_controller.png

Требования:
- Формат: PNG с прозрачным фоном (RGBA, без сжатия с потерями).
- Рекомендуемый размер: 16x16 или 32x32 пикселей (допускаются кратные размеры 64x64 / 128x128).
- Отрисовка в интерфейсе использует pixel-art режим (image-rendering: pixelated) без размытия.
- Файлы в этой папке имеют наивысший приоритет над временными оригиналами из assets-local/ и встроенными заглушками.

Вы также можете загрузить замену прямо из интерфейса приложения («Текстуры -> Заменить PNG»).
