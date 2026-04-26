# Checklist Triển Khai Trích Xuất Theo Nhóm Lớn

Checklist này chỉ dùng cho phần code liên quan đến trích xuất thông tin trong truyện theo các nhóm lớn. Không triển khai UI phức tạp, graph, map, timeline, review nâng cao hoặc translation trong checklist này.

## Phạm Vi Cố Định

- [ ] Chỉ xử lý dữ liệu từ một chương hiện tại.
- [ ] Không dùng future chapter làm evidence.
- [ ] Không hardcode field nhỏ như `tính cách`, `chức vụ`, `khả năng`, `tên gọi khác` trong code xử lý.
- [ ] Cho phép hardcode danh sách nhóm lớn ổn định bằng `group_key`.
- [ ] Prompt hướng dẫn AI tự tạo `field_key`, `field_label` và `values` theo nội dung chương.
- [ ] Output JSON bắt buộc mỗi record phải thuộc đúng một nhóm lớn hợp lệ.
- [ ] Mỗi value nên có confidence và evidence khi có thể.
- [ ] Dữ liệu chưa chắc chắn phải đưa vào nhóm `review_note`.

## Nhóm Lớn Cần Hỗ Trợ

- [ ] `character` - Nhân Vật.
- [ ] `location` - Địa Điểm.
- [ ] `item` - Vật Phẩm.
- [ ] `organization` - Tổ Chức.
- [ ] `species` - Chủng Tộc Và Loài.
- [ ] `ability` - Năng Lực Và Kỹ Thuật.
- [ ] `event` - Sự Kiện.
- [ ] `relationship` - Quan Hệ.
- [ ] `concept` - Khái Niệm Và Thuật Ngữ.
- [ ] `time_marker` - Thời Gian Và Mốc Truyện.
- [ ] `objective` - Nhiệm Vụ Và Mục Tiêu.
- [ ] `review_note` - Ghi Nhận Cần Kiểm Tra.

## Bước 1 - Chốt Contract JSON

- [x] Chốt root shape cho slice `character`:

```json
{
  "schema_version": "story_character_extraction.v1",
  "chapter_num": 1,
  "records": []
}
```

- [x] Chốt shape cho mỗi record:

```json
{
  "group_key": "character",
  "group_label": "Nhân Vật",
  "entity_key": "han_lap",
  "display_name": "Hàn Lập",
  "fields": []
}
```

- [x] Chốt shape cho mỗi field nhỏ do AI tự nhận định:

```json
{
  "field_key": "other_name",
  "field_label": "Tên gọi khác",
  "values": []
}
```

- [x] Chốt shape cho mỗi value:

```json
{
  "value": "Anh ngốc",
  "confidence": 0.86,
  "evidence": [
    {
      "chapter_num": 1,
      "start_char": 120,
      "end_char": 128,
      "quote": "Anh ngốc",
      "reason": "Nhân vật được gọi bằng cách này trong lời thoại."
    }
  ]
}
```

- [x] Quy định `group_key` phải thuộc danh sách nhóm lớn hợp lệ.
- [x] Quy định `field_key` là snake_case do AI đề xuất, backend có thể normalize.
- [x] Quy định `field_label` là tiếng Việt có dấu để hiển thị UI.
- [x] Quy định `display_name` là tên hiển thị tốt nhất mà AI nhận định trong chương.
- [x] Quy định `entity_key` là định danh tạm, chưa phải ID DB chính thức.

## Bước 2 - Chỉnh Prompt Trích Xuất

- [x] Đổi prompt từ schema draft cũ sang schema nhóm lớn cho slice `character`.
- [x] Yêu cầu AI trả về JSON hợp lệ duy nhất, không kèm giải thích ngoài JSON.
- [x] Yêu cầu AI chỉ dùng chương hiện tại làm evidence.
- [x] Yêu cầu AI phân loại mỗi record vào đúng một `group_key`.
- [x] Yêu cầu AI không tự bịa field nếu chương không có dấu hiệu.
- [x] Yêu cầu AI tạo field nhỏ tự nhiên theo nội dung truyện.
- [ ] Yêu cầu AI đưa dữ liệu mơ hồ vào `review_note`.
- [x] Yêu cầu AI trả `group_label`, `field_label` bằng tiếng Việt có dấu.
- [ ] Không hardcode các field nhỏ trong code; chỉ mô tả bằng hướng dẫn trong prompt.

## Bước 3 - Validate Output

- [x] Parse JSON response từ LLM.
- [x] Validate `schema_version`.
- [x] Validate `records` là array.
- [x] Validate `group_key` thuộc nhóm lớn hợp lệ.
- [x] Validate `group_label` không rỗng.
- [ ] Validate `entity_key` không rỗng nếu record là thực thể có thể định danh.
- [x] Validate `display_name` không rỗng nếu record là thực thể có thể định danh.
- [x] Validate `fields` là array.
- [x] Validate mỗi `field_key` không rỗng.
- [x] Validate mỗi `field_label` không rỗng.
- [x] Validate mỗi `values` là array.
- [x] Validate `confidence` nằm trong khoảng `0..1` nếu có.
- [x] Validate evidence span nằm trong bounds của chapter text nếu có `start_char` và `end_char`.
- [ ] Validate `quote` có thể đối chiếu với chapter text khi có thể.
- [x] Nếu JSON lỗi, trả lỗi rõ ràng và để job pause/fail theo trạng thái hiện có.

## Bước 4 - Lưu Dữ Liệu Tối Thiểu

- [x] Không chỉ lưu raw LLM blob làm nguồn sự thật chính.
- [x] Thêm model lưu record trích xuất theo nhóm lớn.
- [x] Lưu `project_id`, `novel_id`, `chapter_id`, `chapter_num`, `job_id`, `run_id`.
- [x] Lưu `group_key`, `group_label`, `entity_key`, `display_name`.
- [x] Lưu `field_key`, `field_label`, `value`, `confidence`.
- [x] Lưu evidence span hoặc evidence payload tối thiểu.
- [x] Lưu provider/model/prompt schema version để debug.
- [x] Ghi event bền vững sau khi dữ liệu trích xuất nhân vật được persist để UI có thể đồng bộ realtime.
- [ ] Cho phép một chương có nhiều record cùng nhóm lớn.
- [ ] Cho phép một record có nhiều field nhỏ.
- [ ] Cho phép một field có nhiều value.
- [ ] Chưa cần merge entity xuyên chương ở bước này.

## Bước 5 - Tích Hợp Vào Analysis Runner Hiện Có

- [x] Khi chạy một chương, gọi prompt schema nhóm lớn mới.
- [x] Chia một chương thành nhiều đoạn nhỏ để giảm lỗi JSON khi gọi local LLM.
- [x] Với mỗi đoạn nhỏ, tách thành nhiều pass nhỏ thay vì dồn vào một prompt lớn.
- [x] Pass 1 trích xuất `name` và `aliases`, sau đó ghi bản nháp nhân vật vào DB.
- [x] Pass 2 đọc nhân vật/alias đã ghi trong DB rồi trích xuất mention offsets cho từng nhân vật.
- [x] Pass 3 đọc nhân vật/alias đã ghi trong DB rồi trích xuất field/fact nhỏ cho từng nhân vật.
- [x] Tắt thinking của llama.cpp trong các pass JSON để model local nhỏ trả `content` trực tiếp.
- [x] Nếu LLM trả JSON hợp lệ, validate và lưu dữ liệu đã parse.
- [x] Tự sửa offset mention bằng cách đối chiếu `mention.text` trong đoạn nhỏ; nếu không đối chiếu được thì bỏ riêng mention đó.
- [x] Quy đổi `start_char` và `end_char` từ offset trong đoạn nhỏ về offset toàn chương trước khi lưu DB.
- [x] Merge record nhân vật từ nhiều đoạn nhỏ trước khi persist để hạn chế trùng dữ liệu.
- [x] Đánh dấu `analysis_chapter_runs.status = completed` sau khi lưu parse thành công.
- [x] Sau khi lưu parse thành công, Reading workspace có đường đồng bộ tự động để thấy highlight mới mà không cần refresh thủ công.
- [x] Nếu parse lỗi, lưu lỗi vào chapter run và pause job để người dùng thấy.
- [x] Resume vẫn bỏ qua chương đã completed.
- [x] Force rerun xóa hoặc thay thế dữ liệu trích xuất của job/chapter hiện tại theo policy rõ ràng.
- [x] Không triển khai xử lý song song nhiều chương trong bước này.

## Bước 6 - API Đọc Kết Quả Tối Thiểu

- [ ] Thêm API đọc kết quả trích xuất theo project/job/chapter.
- [ ] API trả về records đã lưu theo nhóm lớn.
- [ ] API không cần UI đẹp ở bước này, chỉ cần đủ để kiểm tra thủ công.
- [ ] Response giữ nguyên `group_key`, `group_label`, `field_key`, `field_label`, `values`.
- [ ] API phân biệt dữ liệu parsed và raw LLM debug payload.

## Bước 7 - Kiểm Tra Thủ Công

- [ ] Import một truyện ngắn hoặc vài chương mẫu.
- [ ] Chạy analysis cho một chương.
- [ ] Kiểm tra job chuyển sang `completed`.
- [ ] Kiểm tra DB có record theo nhóm lớn.
- [ ] Kiểm tra `Nhân Vật` có tên/alias/field nhỏ do AI tự tạo.
- [ ] Kiểm tra `Địa Điểm` chỉ xuất hiện khi chương có địa điểm.
- [ ] Kiểm tra `Vật Phẩm` chỉ xuất hiện khi chương có vật phẩm.
- [ ] Kiểm tra dữ liệu mơ hồ được đưa vào `review_note`.
- [ ] Kiểm tra evidence không dùng text ngoài chương hiện tại.
- [ ] Kiểm tra không có field nhỏ bị hardcode trong code xử lý.

## Không Làm Trong Slice Này

- [ ] Không làm UI chỉnh sửa inline.
- [ ] Không làm graph/map/timeline renderer.
- [ ] Không làm entity merge xuyên chương.
- [ ] Không làm translation.
- [ ] Không làm RAG/chat.
- [ ] Không làm review queue đầy đủ.
- [ ] Không chạy test rộng nếu người dùng chưa yêu cầu.
